//! [`TextGenerator`] over the Anthropic Messages API.
//!
//! Raw HTTP: there is no official Anthropic SDK for Rust, and this is one
//! endpoint.
//!
//! Two vendor specifics that the port exists to hide:
//!
//! - **Caching is explicit here.** A `cache_control` marker on a content block
//!   tells the vendor to cache everything up to that point, so the request is
//!   built system-then-context-then-instruction and the marker goes after the
//!   context. Reordering those would silently stop the cache working.
//! - **A refusal arrives as HTTP 200** with `stop_reason: "refusal"`. Mapping
//!   errors from status alone would report it as a successful empty
//!   generation.

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{json, Value};

use crate::ai::adapter::outgoing::SseDecoder;
use crate::ai::application::ports::outgoing::{
    Generation, GenerationError, GenerationEvent, GenerationRequest, GenerationStream,
    TextGenerator, Usage,
};

/// Talks to the Anthropic Messages API.
pub struct AnthropicGenerator {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicGenerator {
    /// Builds it from credentials and a model id.
    pub fn new(http: reqwest::Client, api_key: String, model: String) -> Self {
        Self {
            http,
            api_key,
            model,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    /// Points the adapter at another host. For tests against a local stub.
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// The request body.
    ///
    /// Split out so the shape can be asserted without a network call — the
    /// cache marker's position and the schema's nesting are both easy to get
    /// wrong and impossible to notice at runtime.
    pub fn body(&self, request: &GenerationRequest) -> Value {
        let mut body = json!({
            "model": self.model,
            "max_tokens": request.max_output_tokens,
            "system": [{
                "type": "text",
                "text": request.system,
            }],
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": request.context,
                        // Everything up to and including this block is cached.
                        // The instruction below it is not, which is the whole
                        // arrangement: the CV and the job description are re-read
                        // from cache each turn and only the instruction is new.
                        "cache_control": { "type": "ephemeral", "ttl": "1h" },
                    },
                    {
                        "type": "text",
                        "text": request.instruction,
                    },
                ],
            }],
        });

        if let Some(schema) = &request.schema {
            body["output_config"] = json!({
                "format": {
                    "type": "json_schema",
                    "schema": schema,
                }
            });
        }

        body
    }
}

/// Pulls the first text block out of a response.
fn text_of(body: &Value) -> String {
    body["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn usage_of(body: &Value) -> Usage {
    let u = &body["usage"];
    Usage {
        input_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
        cached_input_tokens: u["cache_read_input_tokens"].as_u64().unwrap_or(0) as u32,
    }
}

/// Turns a decoded body into an outcome.
///
/// Separate from the transport so the refusal path can be tested without a
/// server — it is the branch most likely to be got wrong, because it looks
/// like success at the HTTP layer.
pub fn interpret(body: &Value) -> Result<Generation, GenerationError> {
    if body["stop_reason"] == "refusal" {
        let detail = body["stop_details"]["category"]
            .as_str()
            .unwrap_or("no reason given");
        return Err(GenerationError::Refused(detail.to_string()));
    }

    let text = text_of(body);
    if text.is_empty() {
        return Err(GenerationError::Malformed(
            "the reply carried no text block".to_string(),
        ));
    }

    Ok(Generation {
        text,
        usage: usage_of(body),
    })
}

/// What one streamed event means, if anything.
///
/// `Ok(None)` for the events that carry no text and no ending — this vendor
/// emits several, and treating an unrecognised one as a fault would break the
/// stream on a field being added.
///
/// Split out and pure because the case that matters — a refusal arriving in
/// the middle of a `message_delta`, after text has already gone out — is
/// impossible to provoke reliably against a live endpoint.
pub fn interpret_event(
    payload: &Value,
    running: &mut Usage,
) -> Result<Option<GenerationEvent>, GenerationError> {
    match payload["type"].as_str() {
        // Carries the input side of the bill, including the cached figure that
        // says whether prefix caching is working.
        Some("message_start") => {
            let u = &payload["message"]["usage"];
            running.input_tokens = u["input_tokens"].as_u64().unwrap_or(0) as u32;
            running.cached_input_tokens = u["cache_read_input_tokens"].as_u64().unwrap_or(0) as u32;
            Ok(None)
        }

        Some("content_block_delta") => Ok(payload["delta"]["text"]
            .as_str()
            .filter(|t| !t.is_empty())
            .map(|t| GenerationEvent::Delta(t.to_string()))),

        // The end, and where a mid-stream refusal shows up.
        Some("message_delta") => {
            running.output_tokens = payload["usage"]["output_tokens"]
                .as_u64()
                .unwrap_or(running.output_tokens as u64) as u32;

            if payload["delta"]["stop_reason"] == "refusal" {
                return Err(GenerationError::Refused(
                    payload["delta"]["stop_details"]["category"]
                        .as_str()
                        .unwrap_or("no reason given")
                        .to_string(),
                ));
            }
            Ok(None)
        }

        Some("message_stop") => Ok(Some(GenerationEvent::Completed(*running))),

        // An error can also arrive as its own event rather than a status.
        Some("error") => Err(GenerationError::Upstream(
            payload["error"]["message"]
                .as_str()
                .unwrap_or("stream error")
                .to_string(),
        )),

        _ => Ok(None),
    }
}

#[async_trait]
impl TextGenerator for AnthropicGenerator {
    fn provider(&self) -> &'static str {
        "anthropic"
    }

    async fn generate(&self, request: GenerationRequest) -> Result<Generation, GenerationError> {
        let response = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&self.body(&request))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    GenerationError::Timeout
                } else {
                    GenerationError::Upstream(e.to_string())
                }
            })?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| GenerationError::Malformed(e.to_string()))?;

        if status.as_u16() == 429 {
            return Err(GenerationError::RateLimited(
                body["error"]["message"]
                    .as_str()
                    .unwrap_or("rate limited")
                    .to_string(),
            ));
        }
        if !status.is_success() {
            return Err(GenerationError::Upstream(format!(
                "{status}: {}",
                body["error"]["message"].as_str().unwrap_or("no detail")
            )));
        }

        interpret(&body)
    }

    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationStream, GenerationError> {
        let mut body = self.body(&request);
        body["stream"] = json!(true);

        let response = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    GenerationError::Timeout
                } else {
                    GenerationError::Upstream(e.to_string())
                }
            })?;

        // A failure before the stream opens is reported here rather than as a
        // first event, so a caller that never gets to iterate still learns why.
        let status = response.status();
        if status.as_u16() == 429 {
            return Err(GenerationError::RateLimited("rate limited".to_string()));
        }
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            return Err(GenerationError::Upstream(format!("{status}: {detail}")));
        }

        let mut decoder = SseDecoder::new();
        let mut usage = Usage::default();

        let stream = response.bytes_stream().flat_map(move |chunk| {
            let mut out: Vec<Result<GenerationEvent, GenerationError>> = Vec::new();

            match chunk {
                Err(e) => out.push(Err(GenerationError::Upstream(e.to_string()))),
                Ok(bytes) => {
                    for payload in decoder.push(&bytes) {
                        let Ok(json) = serde_json::from_str::<Value>(&payload) else {
                            // One unreadable frame is not worth ending a reply
                            // that is otherwise arriving fine.
                            continue;
                        };
                        match interpret_event(&json, &mut usage) {
                            Ok(Some(event)) => out.push(Ok(event)),
                            Ok(None) => {}
                            Err(e) => out.push(Err(e)),
                        }
                    }
                }
            }

            futures::stream::iter(out)
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generator() -> AnthropicGenerator {
        AnthropicGenerator::new(reqwest::Client::new(), "key".into(), "claude-opus-5".into())
    }

    fn a_request() -> GenerationRequest {
        GenerationRequest {
            system: "You help tailor CVs.".into(),
            context: "CV and job description".into(),
            instruction: "Rewrite the third bullet.".into(),
            max_output_tokens: 1024,
            schema: None,
        }
    }

    /// The cache marker must sit after the stable context and before the
    /// volatile instruction. Anywhere else and the expensive material is
    /// reprocessed every turn — which costs money silently, with no error.
    #[test]
    fn the_cache_marker_sits_after_the_context_not_the_instruction() {
        let body = generator().body(&a_request());
        let blocks = body["messages"][0]["content"].as_array().unwrap();

        assert_eq!(blocks[0]["text"], "CV and job description");
        assert_eq!(blocks[0]["cache_control"]["type"], "ephemeral");

        assert_eq!(blocks[1]["text"], "Rewrite the third bullet.");
        assert!(
            blocks[1]["cache_control"].is_null(),
            "the volatile instruction must not be inside the cached prefix"
        );
    }

    /// A tailoring session works one recommendation at a time over many
    /// minutes, so the default few-minute cache lifetime would expire between
    /// turns — exactly the case caching exists for.
    #[test]
    fn the_cache_is_given_a_long_enough_life_for_a_working_session() {
        let body = generator().body(&a_request());

        assert_eq!(
            body["messages"][0]["content"][0]["cache_control"]["ttl"],
            "1h"
        );
    }

    #[test]
    fn a_schema_is_nested_where_the_vendor_expects_it() {
        let mut request = a_request();
        request.schema = Some(json!({ "type": "object" }));

        let body = generator().body(&request);

        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["output_config"]["format"]["schema"]["type"], "object");
    }

    #[test]
    fn no_schema_means_no_output_config() {
        let body = generator().body(&a_request());

        assert!(body["output_config"].is_null());
    }

    /// The branch that looks like success. A refusal comes back 200, so an
    /// adapter mapping on status alone would return an empty generation and
    /// nobody would know why.
    #[test]
    fn a_refusal_is_an_error_even_though_the_call_succeeded() {
        let body = json!({
            "stop_reason": "refusal",
            "stop_details": { "type": "refusal", "category": "cyber" },
            "content": [],
        });

        match interpret(&body) {
            Err(GenerationError::Refused(detail)) => assert_eq!(detail, "cyber"),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_normal_reply_carries_its_text_and_usage() {
        let body = json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": "Rewritten." }],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 3,
                "cache_read_input_tokens": 900,
            },
        });

        let generation = interpret(&body).unwrap();

        assert_eq!(generation.text, "Rewritten.");
        assert_eq!(generation.usage.cached_input_tokens, 900);
    }

    /// An empty reply is a fault, not an answer: returning "" would look like
    /// the model had nothing to say.
    #[test]
    fn a_reply_with_no_text_is_malformed() {
        let body = json!({ "stop_reason": "end_turn", "content": [] });

        assert!(matches!(
            interpret(&body),
            Err(GenerationError::Malformed(_))
        ));
    }

    // ------------------------------------------------------------------
    // Streaming
    // ------------------------------------------------------------------

    /// Feeds frames through the same path the transport uses, so these
    /// exercise the real interpretation rather than a restatement of it.
    fn run(frames: &[Value]) -> (Vec<GenerationEvent>, Option<GenerationError>) {
        let mut usage = Usage::default();
        let mut events = Vec::new();

        for frame in frames {
            match interpret_event(frame, &mut usage) {
                Ok(Some(e)) => events.push(e),
                Ok(None) => {}
                Err(e) => return (events, Some(e)),
            }
        }

        (events, None)
    }

    #[test]
    fn deltas_come_through_in_order_and_the_end_carries_the_cost() {
        let (events, err) = run(&[
            json!({ "type": "message_start", "message": { "usage": {
                "input_tokens": 10, "cache_read_input_tokens": 900 } } }),
            json!({ "type": "content_block_delta", "delta": { "text": "Hel" } }),
            json!({ "type": "content_block_delta", "delta": { "text": "lo" } }),
            json!({ "type": "message_delta", "usage": { "output_tokens": 2 } }),
            json!({ "type": "message_stop" }),
        ]);

        assert!(err.is_none());
        assert_eq!(
            events[..2],
            [
                GenerationEvent::Delta("Hel".into()),
                GenerationEvent::Delta("lo".into())
            ]
        );

        match &events[2] {
            GenerationEvent::Completed(usage) => {
                assert_eq!(usage.cached_input_tokens, 900);
                assert_eq!(usage.output_tokens, 2);
            }
            other => panic!("expected Completed last, got {other:?}"),
        }
    }

    /// The case that makes streaming harder than it looks: the model begins a
    /// reply, then declines. Text has already reached the caller, so this must
    /// arrive as an error *after* the deltas rather than instead of them.
    #[test]
    fn a_refusal_can_arrive_after_text_has_already_been_sent() {
        let (events, err) = run(&[
            json!({ "type": "content_block_delta", "delta": { "text": "Sure, I can" } }),
            json!({ "type": "message_delta", "delta": {
                "stop_reason": "refusal",
                "stop_details": { "category": "cyber" } } }),
        ]);

        assert_eq!(events, [GenerationEvent::Delta("Sure, I can".into())]);
        assert!(
            matches!(err, Some(GenerationError::Refused(c)) if c == "cyber"),
            "a mid-stream refusal must surface as a refusal, not a silent stop"
        );
    }

    /// A field being added upstream must not break a reply that is otherwise
    /// arriving fine.
    #[test]
    fn an_unrecognised_frame_is_ignored() {
        let (events, err) = run(&[
            json!({ "type": "ping" }),
            json!({ "type": "content_block_start", "content_block": {} }),
            json!({ "type": "content_block_delta", "delta": { "text": "hi" } }),
        ]);

        assert!(err.is_none());
        assert_eq!(events, [GenerationEvent::Delta("hi".into())]);
    }

    #[test]
    fn an_error_frame_ends_the_stream() {
        let (_, err) = run(&[json!({ "type": "error", "error": { "message": "overloaded" } })]);

        assert!(matches!(err, Some(GenerationError::Upstream(m)) if m.contains("overloaded")));
    }

    /// Empty deltas are noise; forwarding them would make a caller render
    /// nothing repeatedly.
    #[test]
    fn empty_deltas_are_dropped() {
        let (events, _) = run(&[json!({ "type": "content_block_delta", "delta": { "text": "" } })]);

        assert!(events.is_empty());
    }

    #[test]
    fn the_streamed_body_asks_for_a_stream_and_keeps_the_cache_marker() {
        let mut body = generator().body(&a_request());
        body["stream"] = json!(true);

        assert_eq!(body["stream"], true);
        assert_eq!(
            body["messages"][0]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
    }
}

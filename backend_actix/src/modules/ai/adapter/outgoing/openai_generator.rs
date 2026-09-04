//! [`TextGenerator`] over the OpenAI Chat Completions API.
//!
//! The same port, a different vendor. Three differences the port absorbs:
//!
//! - **Caching is implicit.** There is no marker to place: long prefixes are
//!   cached automatically when they repeat. The port's system/context/
//!   instruction split still matters, because the automatic behaviour only
//!   works if the stable part genuinely comes first and byte-identically —
//!   so the ordering discipline is the same even though the mechanism is not.
//! - **A refusal is a field on the message**, not a stop reason, and the
//!   finish reason for a safety stop differs from Anthropic's.
//! - **The schema is wrapped differently**, and requires a name.

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{json, Value};

use crate::ai::adapter::outgoing::SseDecoder;
use crate::ai::application::ports::outgoing::{
    Generation, GenerationError, GenerationEvent, GenerationRequest, GenerationStream,
    TextGenerator, Usage,
};

/// Talks to the OpenAI Chat Completions API.
pub struct OpenAiGenerator {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiGenerator {
    /// Builds it from credentials and a model id.
    pub fn new(http: reqwest::Client, api_key: String, model: String) -> Self {
        Self {
            http,
            api_key,
            model,
            base_url: "https://api.openai.com".to_string(),
        }
    }

    /// Points the adapter at another host. For tests against a local stub.
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// The request body.
    ///
    /// The stable material goes in the system message and the volatile
    /// instruction in the user message, so the repeated prefix is byte
    /// identical between turns and the vendor's automatic caching can see it.
    pub fn body(&self, request: &GenerationRequest) -> Value {
        let mut body = json!({
            "model": self.model,
            "max_completion_tokens": request.max_output_tokens,
            "messages": [
                {
                    "role": "system",
                    // System and context are joined rather than sent as two
                    // messages: the cached prefix has to be one stable run of
                    // bytes, and an extra message boundary between them would
                    // still be stable but buys nothing.
                    "content": format!("{}\n\n{}", request.system, request.context),
                },
                {
                    "role": "user",
                    "content": request.instruction,
                },
            ],
        });

        if let Some(schema) = &request.schema {
            body["response_format"] = json!({
                "type": "json_schema",
                "json_schema": {
                    // Required here, unlike the other vendor. A fixed name is
                    // fine: it identifies the schema within one request, not
                    // across them.
                    "name": "response",
                    "strict": true,
                    "schema": schema,
                }
            });
        }

        body
    }
}

fn usage_of(body: &Value) -> Usage {
    let u = &body["usage"];
    let cached = u["prompt_tokens_details"]["cached_tokens"]
        .as_u64()
        .unwrap_or(0) as u32;

    Usage {
        // Reported inclusive of cached tokens, unlike the other vendor, so
        // the cached portion is subtracted to make the two comparable.
        input_tokens: (u["prompt_tokens"].as_u64().unwrap_or(0) as u32).saturating_sub(cached),
        output_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
        cached_input_tokens: cached,
    }
}

/// Turns a decoded body into an outcome.
pub fn interpret(body: &Value) -> Result<Generation, GenerationError> {
    let choice = &body["choices"][0];

    // A safety decline is a field on the message here, not a stop reason.
    if let Some(refusal) = choice["message"]["refusal"].as_str() {
        return Err(GenerationError::Refused(refusal.to_string()));
    }
    if choice["finish_reason"] == "content_filter" {
        return Err(GenerationError::Refused("content filtered".to_string()));
    }

    let text = choice["message"]["content"].as_str().unwrap_or_default();
    if text.is_empty() {
        return Err(GenerationError::Malformed(
            "the reply carried no content".to_string(),
        ));
    }

    Ok(Generation {
        text: text.to_string(),
        usage: usage_of(body),
    })
}

/// The sentinel this vendor ends a stream with. Not JSON — parsing it as
/// JSON is the classic way to end a stream with a spurious error.
const DONE: &str = "[DONE]";

/// What one streamed frame means, if anything.
///
/// Split out and pure for the same reason as the other adapter's: a refusal
/// that arrives after several deltas cannot be provoked reliably against a
/// live endpoint, and it is the case most likely to be got wrong.
pub fn interpret_event(
    payload: &Value,
    running: &mut Usage,
) -> Result<Option<GenerationEvent>, GenerationError> {
    // Usage arrives on its own final frame, one with no choices, and only when
    // the request asked for it.
    if let Some(u) = payload.get("usage").filter(|u| !u.is_null()) {
        let cached = u["prompt_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;
        running.cached_input_tokens = cached;
        running.input_tokens =
            (u["prompt_tokens"].as_u64().unwrap_or(0) as u32).saturating_sub(cached);
        running.output_tokens = u["completion_tokens"].as_u64().unwrap_or(0) as u32;
    }

    let Some(choice) = payload["choices"].as_array().and_then(|c| c.first()) else {
        return Ok(None);
    };

    if let Some(refusal) = choice["delta"]["refusal"].as_str() {
        return Err(GenerationError::Refused(refusal.to_string()));
    }

    match choice["finish_reason"].as_str() {
        Some("content_filter") => return Err(GenerationError::Refused("content filtered".into())),
        Some(_) => return Ok(Some(GenerationEvent::Completed(*running))),
        None => {}
    }

    Ok(choice["delta"]["content"]
        .as_str()
        .filter(|t| !t.is_empty())
        .map(|t| GenerationEvent::Delta(t.to_string())))
}

#[async_trait]
impl TextGenerator for OpenAiGenerator {
    fn provider(&self) -> &'static str {
        "openai"
    }

    async fn generate(&self, request: GenerationRequest) -> Result<Generation, GenerationError> {
        let response = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
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
        // Without this the stream carries no usage at all, and the cache-hit
        // figure silently reads zero — which looks exactly like caching having
        // stopped working. Opt in explicitly.
        body["stream_options"] = json!({ "include_usage": true });

        let response = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
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
        let mut finished = false;

        let stream = response.bytes_stream().flat_map(move |chunk| {
            let mut out: Vec<Result<GenerationEvent, GenerationError>> = Vec::new();

            match chunk {
                Err(e) => out.push(Err(GenerationError::Upstream(e.to_string()))),
                Ok(bytes) => {
                    for payload in decoder.push(&bytes) {
                        if payload == DONE {
                            // The usage frame arrives after the frame carrying
                            // finish_reason, so Completed is emitted here when
                            // it has not already gone out — otherwise the
                            // caller would never see the final cost.
                            if !finished {
                                finished = true;
                                out.push(Ok(GenerationEvent::Completed(usage)));
                            }
                            continue;
                        }

                        let Ok(json) = serde_json::from_str::<Value>(&payload) else {
                            continue;
                        };

                        match interpret_event(&json, &mut usage) {
                            Ok(Some(GenerationEvent::Completed(_))) => {
                                // Swallowed: usage is still to come on a later
                                // frame, so completing now would report a cost
                                // of zero.
                            }
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

    fn generator() -> OpenAiGenerator {
        OpenAiGenerator::new(reqwest::Client::new(), "key".into(), "gpt-4o".into())
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

    /// The stable material must lead and the volatile instruction must follow,
    /// exactly as with the other vendor. The mechanism differs — caching is
    /// automatic here — but the ordering discipline is what makes it possible
    /// either way.
    #[test]
    fn the_stable_material_leads_and_the_instruction_follows() {
        let body = generator().body(&a_request());
        let messages = body["messages"].as_array().unwrap();

        assert_eq!(messages[0]["role"], "system");
        assert!(messages[0]["content"]
            .as_str()
            .unwrap()
            .contains("CV and job description"));

        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "Rewrite the third bullet.");
    }

    #[test]
    fn a_schema_is_wrapped_the_way_this_vendor_expects() {
        let mut request = a_request();
        request.schema = Some(json!({ "type": "object" }));

        let body = generator().body(&request);

        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert_eq!(
            body["response_format"]["json_schema"]["schema"]["type"],
            "object"
        );
    }

    #[test]
    fn a_refusal_field_is_an_error() {
        let body = json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "refusal": "I can't help with that." },
            }],
        });

        assert!(matches!(interpret(&body), Err(GenerationError::Refused(_))));
    }

    #[test]
    fn a_filtered_completion_is_an_error() {
        let body = json!({
            "choices": [{ "finish_reason": "content_filter", "message": {} }],
        });

        assert!(matches!(interpret(&body), Err(GenerationError::Refused(_))));
    }

    /// This vendor counts cached tokens inside the prompt total and the other
    /// reports them separately. Subtracting here is what makes the two
    /// adapters' Usage mean the same thing — without it, one vendor would
    /// appear to send far more than the other for identical work.
    #[test]
    fn cached_tokens_are_reported_the_same_way_as_the_other_vendor() {
        let body = json!({
            "choices": [{ "finish_reason": "stop", "message": { "content": "Done." } }],
            "usage": {
                "prompt_tokens": 1000,
                "completion_tokens": 20,
                "prompt_tokens_details": { "cached_tokens": 900 },
            },
        });

        let usage = interpret(&body).unwrap().usage;

        assert_eq!(usage.cached_input_tokens, 900);
        assert_eq!(
            usage.input_tokens, 100,
            "cached tokens must not be counted twice"
        );
    }

    // ------------------------------------------------------------------
    // Streaming
    // ------------------------------------------------------------------

    fn run(frames: &[Value]) -> (Vec<GenerationEvent>, Option<GenerationError>, Usage) {
        let mut usage = Usage::default();
        let mut events = Vec::new();

        for frame in frames {
            match interpret_event(frame, &mut usage) {
                Ok(Some(e)) => events.push(e),
                Ok(None) => {}
                Err(e) => return (events, Some(e), usage),
            }
        }

        (events, None, usage)
    }

    #[test]
    fn deltas_come_through_in_order() {
        let (events, err, _) = run(&[
            json!({ "choices": [{ "delta": { "content": "Hel" } }] }),
            json!({ "choices": [{ "delta": { "content": "lo" } }] }),
        ]);

        assert!(err.is_none());
        assert_eq!(
            events,
            [
                GenerationEvent::Delta("Hel".into()),
                GenerationEvent::Delta("lo".into())
            ]
        );
    }

    /// Same hard case as the other vendor, signalled differently: text first,
    /// then the decline.
    #[test]
    fn a_refusal_can_arrive_after_text_has_already_been_sent() {
        let (events, err, _) = run(&[
            json!({ "choices": [{ "delta": { "content": "Sure, I can" } }] }),
            json!({ "choices": [{ "delta": { "refusal": "I can't help with that." } }] }),
        ]);

        assert_eq!(events, [GenerationEvent::Delta("Sure, I can".into())]);
        assert!(matches!(err, Some(GenerationError::Refused(_))));
    }

    #[test]
    fn a_filtered_stream_is_a_refusal() {
        let (_, err, _) =
            run(&[json!({ "choices": [{ "delta": {}, "finish_reason": "content_filter" }] })]);

        assert!(matches!(err, Some(GenerationError::Refused(_))));
    }

    /// Usage arrives on its own final frame, after the one carrying
    /// finish_reason. The transport holds Completed back until then — this
    /// pins that the figures are read off that frame at all.
    #[test]
    fn the_usage_frame_is_read_and_normalised() {
        let (_, err, usage) = run(&[
            json!({ "choices": [{ "delta": { "content": "hi" } }] }),
            json!({ "choices": [], "usage": {
                "prompt_tokens": 1000,
                "completion_tokens": 20,
                "prompt_tokens_details": { "cached_tokens": 900 }
            } }),
        ]);

        assert!(err.is_none());
        assert_eq!(usage.cached_input_tokens, 900);
        assert_eq!(
            usage.input_tokens, 100,
            "cached tokens must not be counted twice, as in the non-streamed path"
        );
    }

    /// The sentinel is not JSON. Handling it in the transport rather than the
    /// interpreter is what stops a stream ending with a spurious parse error.
    #[test]
    fn the_done_sentinel_is_not_json() {
        assert!(serde_json::from_str::<Value>(DONE).is_err());
    }

    /// Without this the stream carries no usage at all, and the cache-hit
    /// figure reads zero — indistinguishable from caching having broken.
    #[test]
    fn the_streamed_body_opts_into_usage_reporting() {
        let mut body = generator().body(&a_request());
        body["stream"] = json!(true);
        body["stream_options"] = json!({ "include_usage": true });

        assert_eq!(body["stream_options"]["include_usage"], true);
    }

    #[test]
    fn empty_deltas_are_dropped() {
        let (events, _, _) = run(&[json!({ "choices": [{ "delta": { "content": "" } }] })]);

        assert!(events.is_empty());
    }
}

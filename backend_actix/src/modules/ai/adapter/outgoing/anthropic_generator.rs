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
use serde_json::{json, Value};

use crate::ai::application::ports::outgoing::{
    Generation, GenerationError, GenerationRequest, TextGenerator, Usage,
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
}

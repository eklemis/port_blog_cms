//! Choosing a vendor from configuration.
//!
//! The vendor is a deployment decision, not a code one. Nothing above the
//! [`TextGenerator`] port knows or can ask which of these was built.

use std::sync::Arc;

use crate::ai::adapter::outgoing::{AnthropicGenerator, OpenAiGenerator};
use crate::ai::application::ports::outgoing::TextGenerator;

/// Which vendor answers generation requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provider {
    /// Anthropic's Messages API.
    #[default]
    Anthropic,
    /// OpenAI's Chat Completions API.
    OpenAi,
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Ok(Provider::Anthropic),
            "openai" | "gpt" => Ok(Provider::OpenAi),
            other => Err(format!(
                "unknown AI_PROVIDER {other:?}; expected \"anthropic\" or \"openai\""
            )),
        }
    }
}

impl Provider {
    /// The model used when `AI_MODEL` is not set.
    ///
    /// A default per vendor rather than one shared default, because a model id
    /// from one vendor is meaningless to the other — and a wrong id fails at
    /// the first request rather than at startup.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-opus-5",
            Provider::OpenAi => "gpt-4o",
        }
    }
}

/// Why a generator could not be built.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProviderConfigError {
    /// `AI_PROVIDER` named something that is not a supported vendor.
    #[error("{0}")]
    UnknownProvider(String),

    /// No API key was configured for the chosen vendor.
    #[error("AI_API_KEY is not set; generation is disabled")]
    MissingKey,
}

/// Reads the environment and builds whichever vendor it names.
///
/// - `AI_PROVIDER` — `anthropic` (default) or `openai`
/// - `AI_MODEL` — defaults per vendor
/// - `AI_API_KEY` — required
///
/// Returns `Ok(None)` when no key is configured, so a deployment without
/// generation starts normally and the AI routes report the feature as off.
/// Failing to boot the whole API because one optional feature has no
/// credentials would be a worse trade.
pub fn from_env(
    http: reqwest::Client,
) -> Result<Option<Arc<dyn TextGenerator>>, ProviderConfigError> {
    let provider: Provider = match std::env::var("AI_PROVIDER") {
        Ok(raw) if !raw.trim().is_empty() => {
            raw.parse().map_err(ProviderConfigError::UnknownProvider)?
        }
        _ => Provider::default(),
    };

    let key = match std::env::var("AI_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => return Ok(None),
    };

    let model = std::env::var("AI_MODEL")
        .ok()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| provider.default_model().to_string());

    tracing::info!("Generation is enabled: provider={provider:?}, model={model}");

    Ok(Some(match provider {
        Provider::Anthropic => Arc::new(AnthropicGenerator::new(http, key, model)),
        Provider::OpenAi => Arc::new(OpenAiGenerator::new(http, key, model)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn both_vendors_are_selectable_by_name() {
        assert_eq!(Provider::from_str("anthropic"), Ok(Provider::Anthropic));
        assert_eq!(Provider::from_str("openai"), Ok(Provider::OpenAi));
    }

    /// Configuration is typed by humans, so the obvious informal names work
    /// and case does not matter.
    #[test]
    fn the_names_people_actually_type_are_accepted() {
        for (raw, expected) in [
            ("Anthropic", Provider::Anthropic),
            ("claude", Provider::Anthropic),
            ("  OpenAI  ", Provider::OpenAi),
            ("gpt", Provider::OpenAi),
        ] {
            assert_eq!(Provider::from_str(raw), Ok(expected), "for {raw:?}");
        }
    }

    /// An unrecognised vendor is refused rather than silently defaulted.
    /// Quietly using the other vendor's key against the wrong API would fail
    /// far from the cause.
    #[test]
    fn an_unknown_vendor_is_rejected_with_the_options_named() {
        let err = Provider::from_str("gemini").unwrap_err();

        assert!(err.contains("anthropic"), "{err}");
        assert!(err.contains("openai"), "{err}");
    }

    /// A model id from one vendor means nothing to the other, so the default
    /// follows the choice rather than being shared.
    #[test]
    fn each_vendor_brings_its_own_default_model() {
        assert_ne!(
            Provider::Anthropic.default_model(),
            Provider::OpenAi.default_model()
        );
    }
}

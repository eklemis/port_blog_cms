//! Creating a topic.
//!
//! This file is the worked example in `docs/ARCHITECTURE.md`: it shows the
//! shape every incoming port in the newer modules follows — a command whose
//! constructor validates, an error enum per operation, and a trait the route
//! handler depends on.

use async_trait::async_trait;

use crate::{
    auth::application::domain::entities::UserId, topic::application::ports::outgoing::TopicResult,
};

//
// ──────────────────────────────────────────────────────────
// Create Topic Command
// ──────────────────────────────────────────────────────────
//

/// A validated request to create a topic.
///
/// Fields are private and the only constructor is [`new`](Self::new), which
/// returns `Result`. A route handler therefore cannot build an invalid command,
/// and validation lives in the application layer rather than in the handler.
#[derive(Debug, Clone)]
pub struct CreateTopicCommand {
    owner: UserId,
    title: String,
    description: Option<String>,
}

/// Why a command could not be built.
///
/// Separate from [`CreateTopicError`] because these are caught before the use
/// case runs — nothing has been attempted yet.
#[derive(Debug, thiserror::Error)]
pub enum CreateTopicCommandError {
    /// The title was empty once trimmed.
    #[error("Title cannot be empty")]
    EmptyTitle,

    /// The title exceeds 100 characters.
    #[error("Title too long")]
    TitleTooLong,
}

impl CreateTopicCommand {
    /// Validates and builds the command.
    ///
    /// The title is trimmed first, so a whitespace-only title is rejected as
    /// [`EmptyTitle`](CreateTopicCommandError::EmptyTitle) rather than stored.
    pub fn new(
        owner: UserId,
        title: String,
        description: Option<String>,
    ) -> Result<Self, CreateTopicCommandError> {
        let title = title.trim();

        if title.is_empty() {
            return Err(CreateTopicCommandError::EmptyTitle);
        }

        if title.len() > 100 {
            return Err(CreateTopicCommandError::TitleTooLong);
        }

        Ok(Self {
            owner,
            title: title.to_string(),
            description,
        })
    }

    /// The user the topic will belong to.
    pub fn owner(&self) -> &UserId {
        &self.owner
    }

    /// The trimmed, validated title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The optional description, as supplied.
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }
}

//
// ──────────────────────────────────────────────────────────
// Use Case Error
// ──────────────────────────────────────────────────────────
//

/// Why creating a topic failed once the use case ran.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CreateTopicError {
    /// The owner already has a topic with that title.
    #[error("Topic already exists")]
    TopicAlreadyExists,

    /// The store could not be reached.
    #[error("Repository error: {0}")]
    RepositoryError(String),
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

/// Creates a topic.
#[async_trait]
pub trait CreateTopicUseCase: Send + Sync {
    /// Creates the topic and returns it as stored.
    async fn execute(&self, command: CreateTopicCommand) -> Result<TopicResult, CreateTopicError>;
}

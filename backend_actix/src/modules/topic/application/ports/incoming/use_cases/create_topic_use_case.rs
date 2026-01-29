use async_trait::async_trait;

use crate::{
    auth::application::domain::entities::UserId, topic::application::ports::outgoing::TopicResult,
};

//
// ──────────────────────────────────────────────────────────
// Create Topic Command
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone)]
pub struct CreateTopicCommand {
    owner: UserId,
    title: String,
    description: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateTopicCommandError {
    #[error("Title cannot be empty")]
    EmptyTitle,

    #[error("Title too long")]
    TitleTooLong,
}

impl CreateTopicCommand {
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

    pub fn owner(&self) -> &UserId {
        &self.owner
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }
}

//
// ──────────────────────────────────────────────────────────
// Use Case Error
// ──────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, thiserror::Error)]
pub enum CreateTopicError {
    #[error("Topic already exists")]
    TopicAlreadyExists,

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

//
// ──────────────────────────────────────────────────────────
// Incoming Port (Use Case)
// ──────────────────────────────────────────────────────────
//

#[async_trait]
pub trait CreateTopicUseCase: Send + Sync {
    async fn execute(&self, command: CreateTopicCommand) -> Result<TopicResult, CreateTopicError>;
}

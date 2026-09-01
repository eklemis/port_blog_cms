//! What topic needs from the outside: a topic store, split into write and read ports.

mod topic_query;
mod topic_repository;

pub use topic_query::{TopicQuery, TopicQueryError, TopicQueryResult};
pub use topic_repository::{CreateTopicData, TopicRepository, TopicRepositoryError, TopicResult};

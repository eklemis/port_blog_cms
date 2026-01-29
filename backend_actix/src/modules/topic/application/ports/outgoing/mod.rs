mod topic_query;
mod topic_repository;

pub use topic_query::{TopicQuery, TopicQueryError, TopicQueryResult};
pub use topic_repository::{CreateTopicData, TopicRepository, TopicRepositoryError, TopicResult};

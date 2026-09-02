//! Restores an archived project.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::project::application::ports::incoming::use_cases::{
    RestoreProjectError, RestoreProjectUseCase,
};
use crate::project::application::ports::outgoing::project_archiver::ProjectArchiver;

/// Implements the corresponding use-case contract.
///
/// A pass-through: the archiver is already owner-scoped in SQL, so this exists
/// to map the outgoing error onto the one the endpoint speaks.
#[derive(Debug, Clone)]
pub struct RestoreProjectService<A> {
    archiver: A,
}

impl<A> RestoreProjectService<A> {
    /// Builds it from the ports it depends on.
    pub fn new(archiver: A) -> Self {
        Self { archiver }
    }
}

#[async_trait]
impl<A: ProjectArchiver + Send + Sync> RestoreProjectUseCase for RestoreProjectService<A> {
    async fn execute(&self, owner: UserId, project_id: Uuid) -> Result<(), RestoreProjectError> {
        self.archiver
            .restore(owner, project_id)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::application::ports::outgoing::project_archiver::ProjectArchiverError;
    use std::sync::Mutex;

    #[derive(Clone)]
    struct SpyArchiver {
        called: std::sync::Arc<Mutex<Vec<&'static str>>>,
        result: Result<(), ProjectArchiverError>,
    }

    impl SpyArchiver {
        fn new(result: Result<(), ProjectArchiverError>) -> Self {
            Self {
                called: std::sync::Arc::new(Mutex::new(Vec::new())),
                result,
            }
        }
    }

    #[async_trait]
    impl ProjectArchiver for SpyArchiver {
        async fn restore(&self, _o: UserId, _p: Uuid) -> Result<(), ProjectArchiverError> {
            self.called.lock().unwrap().push("restore");
            self.result.clone()
        }
        async fn soft_delete(&self, _o: UserId, _p: Uuid) -> Result<(), ProjectArchiverError> {
            self.called.lock().unwrap().push("soft_delete");
            self.result.clone()
        }
        async fn hard_delete(&self, _o: UserId, _p: Uuid) -> Result<(), ProjectArchiverError> {
            self.called.lock().unwrap().push("hard_delete");
            self.result.clone()
        }
    }

    /// Restore must call restore. Crossing this with hard_delete would turn an
    /// undo into a permanent deletion, and the wiring is one word.
    #[tokio::test]
    async fn it_restores_rather_than_deleting() {
        let archiver = SpyArchiver::new(Ok(()));
        let svc = RestoreProjectService::new(archiver.clone());

        svc.execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap();

        assert_eq!(*archiver.called.lock().unwrap(), vec!["restore"]);
    }

    #[tokio::test]
    async fn a_missing_project_is_not_found_rather_than_a_500() {
        let archiver = SpyArchiver::new(Err(ProjectArchiverError::NotFound));
        let svc = RestoreProjectService::new(archiver.clone());

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(matches!(err, RestoreProjectError::ProjectNotFound));
    }
}

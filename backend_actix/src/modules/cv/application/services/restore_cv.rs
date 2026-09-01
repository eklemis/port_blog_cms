use crate::auth::application::domain::entities::UserId;
use crate::cv::application::ports::outgoing::{
    CVArchiver, CVArchiverError, CVRepository, CVRepositoryError,
};
use crate::cv::application::use_cases::restore_cv::{RestoreCVError, RestoreDeletedCvUseCase};
use crate::cv::domain::entities::CVInfo;
use async_trait::async_trait;
use uuid::Uuid;

pub struct RestoreCvService<A, R>
where
    A: CVArchiver + Send + Sync,
    R: CVRepository + Send + Sync,
{
    cv_archiver: A,
    cv_repository: R,
}

impl<A, R> RestoreCvService<A, R>
where
    A: CVArchiver + Send + Sync,
    R: CVRepository + Send + Sync,
{
    pub fn new(cv_archiver: A, cv_repository: R) -> Self {
        Self {
            cv_archiver,
            cv_repository,
        }
    }
}

#[async_trait]
impl<A, R> RestoreDeletedCvUseCase for RestoreCvService<A, R>
where
    A: CVArchiver + Send + Sync,
    R: CVRepository + Send + Sync,
{
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<CVInfo, RestoreCVError> {
        // `fetch_cv_by_id` does not filter archived rows, which is what makes
        // the ownership check possible for a CV that is currently soft-deleted.
        let cv = self
            .cv_repository
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|e| match e {
                CVRepositoryError::NotFound => RestoreCVError::CVNotFound,
                CVRepositoryError::DatabaseError(msg) => RestoreCVError::RepositoryError(msg),
            })?
            .ok_or(RestoreCVError::CVNotFound)?;

        if cv.user_id != user_id.value() {
            return Err(RestoreCVError::Unauthorized);
        }

        match self.cv_archiver.restore(cv_id).await {
            Ok(restored) => Ok(restored),

            // Not archived means the CV is already in the state the caller
            // wants. Return the copy already fetched rather than erroring, so
            // restore is idempotent.
            Err(CVArchiverError::NotArchived) => Ok(cv),

            Err(CVArchiverError::NotFound) => Err(RestoreCVError::CVNotFound),
            Err(CVArchiverError::AlreadyArchived) => Err(RestoreCVError::RepositoryError(
                "CV is archived".to_string(),
            )),
            Err(CVArchiverError::DatabaseError(msg)) => Err(RestoreCVError::RepositoryError(msg)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CreateCVData, UpdateCVData};

    struct MockCVArchiver {
        result: Result<CVInfo, CVArchiverError>,
    }

    #[async_trait]
    impl CVArchiver for MockCVArchiver {
        async fn soft_delete(&self, _cv_id: Uuid) -> Result<(), CVArchiverError> {
            unimplemented!()
        }
        async fn hard_delete(&self, _cv_id: Uuid) -> Result<(), CVArchiverError> {
            unimplemented!()
        }
        async fn restore(&self, _cv_id: Uuid) -> Result<CVInfo, CVArchiverError> {
            self.result.clone()
        }
    }

    struct MockCVRepository {
        result: Result<Option<CVInfo>, CVRepositoryError>,
    }

    #[async_trait]
    impl CVRepository for MockCVRepository {
        async fn fetch_cv_by_user_id(
            &self,
            _user_id: Uuid,
        ) -> Result<Vec<CVInfo>, CVRepositoryError> {
            unimplemented!()
        }
        async fn fetch_cv_by_id(&self, _cv_id: Uuid) -> Result<Option<CVInfo>, CVRepositoryError> {
            self.result.clone()
        }
        async fn create_cv(
            &self,
            _user_id: Uuid,
            _cv_data: CreateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!()
        }
        async fn update_cv(
            &self,
            _cv_id: Uuid,
            _cv_data: UpdateCVData,
        ) -> Result<CVInfo, CVRepositoryError> {
            unimplemented!()
        }
    }

    fn cv_owned_by(cv_id: Uuid, user_id: Uuid, role: &str) -> CVInfo {
        CVInfo {
            id: cv_id,
            user_id,
            role: role.to_string(),
            display_name: "Test User".to_string(),
            bio: "Test bio".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    fn service(
        archiver: Result<CVInfo, CVArchiverError>,
        repo: Result<Option<CVInfo>, CVRepositoryError>,
    ) -> RestoreCvService<MockCVArchiver, MockCVRepository> {
        RestoreCvService::new(
            MockCVArchiver { result: archiver },
            MockCVRepository { result: repo },
        )
    }

    #[tokio::test]
    async fn restores_an_archived_cv_and_returns_it() {
        let (cv_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let svc = service(
            Ok(cv_owned_by(cv_id, user_id, "restored")),
            Ok(Some(cv_owned_by(cv_id, user_id, "stale"))),
        );

        let restored = svc.execute(UserId::from(user_id), cv_id).await.unwrap();

        // The archiver's copy wins, not the one fetched for the ownership check.
        assert_eq!(restored.role, "restored");
    }

    /// Restoring a CV that was never archived leaves it in the state the caller
    /// wants, so it succeeds and returns the CV rather than erroring.
    #[tokio::test]
    async fn restoring_an_unarchived_cv_is_idempotent() {
        let (cv_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let svc = service(
            Err(CVArchiverError::NotArchived),
            Ok(Some(cv_owned_by(cv_id, user_id, "already active"))),
        );

        let restored = svc.execute(UserId::from(user_id), cv_id).await.unwrap();
        assert_eq!(restored.role, "already active");
    }

    #[tokio::test]
    async fn refuses_to_restore_another_users_cv() {
        let (cv_id, owner, caller) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let svc = service(
            Ok(cv_owned_by(cv_id, owner, "x")),
            Ok(Some(cv_owned_by(cv_id, owner, "x"))),
        );

        let err = svc.execute(UserId::from(caller), cv_id).await.unwrap_err();
        assert!(matches!(err, RestoreCVError::Unauthorized));
    }

    #[tokio::test]
    async fn reports_not_found_when_the_cv_is_missing() {
        let svc = service(Err(CVArchiverError::NotFound), Ok(None));

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, RestoreCVError::CVNotFound));
    }

    #[tokio::test]
    async fn surfaces_archiver_failures() {
        let (cv_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let svc = service(
            Err(CVArchiverError::DatabaseError("db down".into())),
            Ok(Some(cv_owned_by(cv_id, user_id, "x"))),
        );

        let err = svc.execute(UserId::from(user_id), cv_id).await.unwrap_err();
        assert!(matches!(err, RestoreCVError::RepositoryError(m) if m == "db down"));
    }
}

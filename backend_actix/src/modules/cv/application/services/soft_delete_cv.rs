use crate::auth::application::domain::entities::UserId;
use crate::cv::application::ports::outgoing::{
    CVArchiver, CVArchiverError, CVRepository, CVRepositoryError,
};
use crate::cv::application::use_cases::soft_delete_cv::{SoftDeleteCVError, SoftDeleteCvUseCase};
use async_trait::async_trait;
use uuid::Uuid;

pub struct SoftDeleteCvService<A, R>
where
    A: CVArchiver + Send + Sync,
    R: CVRepository + Send + Sync,
{
    cv_archiver: A,
    cv_repository: R,
}

impl<A, R> SoftDeleteCvService<A, R>
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
impl<A, R> SoftDeleteCvUseCase for SoftDeleteCvService<A, R>
where
    A: CVArchiver + Send + Sync,
    R: CVRepository + Send + Sync,
{
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<(), SoftDeleteCVError> {
        // `CVArchiver::soft_delete` takes only a cv_id, so ownership has to be
        // established here rather than pushed into the query.
        let cv = self
            .cv_repository
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|e| match e {
                CVRepositoryError::NotFound => SoftDeleteCVError::CVNotFound,
                CVRepositoryError::DatabaseError(msg) => SoftDeleteCVError::RepositoryError(msg),
            })?
            .ok_or(SoftDeleteCVError::CVNotFound)?;

        if cv.user_id != user_id.value() {
            return Err(SoftDeleteCVError::Unauthorized);
        }

        match self.cv_archiver.soft_delete(cv_id).await {
            Ok(()) => Ok(()),

            // Already archived is the state the caller asked for, so treat a
            // repeated DELETE as success rather than surfacing an error.
            Err(CVArchiverError::AlreadyArchived) => Ok(()),

            Err(CVArchiverError::NotFound) => Err(SoftDeleteCVError::CVNotFound),
            Err(CVArchiverError::NotArchived) => Err(SoftDeleteCVError::RepositoryError(
                "CV is not archived".to_string(),
            )),
            Err(CVArchiverError::DatabaseError(msg)) => {
                Err(SoftDeleteCVError::RepositoryError(msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CreateCVData, UpdateCVData};
    use crate::cv::domain::entities::CVInfo;

    struct MockCVArchiver {
        result: Result<(), CVArchiverError>,
    }

    #[async_trait]
    impl CVArchiver for MockCVArchiver {
        async fn soft_delete(&self, _cv_id: Uuid) -> Result<(), CVArchiverError> {
            self.result.clone()
        }
        async fn hard_delete(&self, _cv_id: Uuid) -> Result<(), CVArchiverError> {
            unimplemented!()
        }
        async fn restore(&self, _cv_id: Uuid) -> Result<CVInfo, CVArchiverError> {
            unimplemented!()
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

    fn cv_owned_by(cv_id: Uuid, user_id: Uuid) -> CVInfo {
        CVInfo {
            id: cv_id,
            user_id,
            role: "Developer".to_string(),
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
        archiver: Result<(), CVArchiverError>,
        repo: Result<Option<CVInfo>, CVRepositoryError>,
    ) -> SoftDeleteCvService<MockCVArchiver, MockCVRepository> {
        SoftDeleteCvService::new(
            MockCVArchiver { result: archiver },
            MockCVRepository { result: repo },
        )
    }

    #[tokio::test]
    async fn archives_a_cv_the_caller_owns() {
        let (cv_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let svc = service(Ok(()), Ok(Some(cv_owned_by(cv_id, user_id))));

        assert!(svc.execute(UserId::from(user_id), cv_id).await.is_ok());
    }

    /// A second DELETE must not fail. The CV is already in the state the caller
    /// asked for, so `AlreadyArchived` is success, not an error.
    #[tokio::test]
    async fn archiving_twice_is_idempotent() {
        let (cv_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let svc = service(
            Err(CVArchiverError::AlreadyArchived),
            Ok(Some(cv_owned_by(cv_id, user_id))),
        );

        assert!(svc.execute(UserId::from(user_id), cv_id).await.is_ok());
    }

    #[tokio::test]
    async fn refuses_to_archive_another_users_cv() {
        let (cv_id, owner, caller) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let svc = service(Ok(()), Ok(Some(cv_owned_by(cv_id, owner))));

        let err = svc.execute(UserId::from(caller), cv_id).await.unwrap_err();
        assert!(matches!(err, SoftDeleteCVError::Unauthorized));
    }

    #[tokio::test]
    async fn reports_not_found_when_the_cv_is_missing() {
        let svc = service(Ok(()), Ok(None));

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, SoftDeleteCVError::CVNotFound));
    }

    #[tokio::test]
    async fn surfaces_repository_failures() {
        let svc = service(
            Ok(()),
            Err(CVRepositoryError::DatabaseError("db down".into())),
        );

        let err = svc
            .execute(UserId::from(Uuid::new_v4()), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, SoftDeleteCVError::RepositoryError(m) if m == "db down"));
    }

    #[tokio::test]
    async fn surfaces_archiver_failures() {
        let (cv_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
        let svc = service(
            Err(CVArchiverError::DatabaseError("archive failed".into())),
            Ok(Some(cv_owned_by(cv_id, user_id))),
        );

        let err = svc.execute(UserId::from(user_id), cv_id).await.unwrap_err();
        assert!(matches!(err, SoftDeleteCVError::RepositoryError(m) if m == "archive failed"));
    }
}

use crate::auth::application::domain::entities::UserId;
use crate::cv::application::ports::outgoing::{
    CVArchiver, CVArchiverError, CVRepository, CVRepositoryError,
};
use crate::cv::application::use_cases::hard_delete_cv::{HardDeleteCVError, HardDeleteCvUseCase};
use async_trait::async_trait;
use uuid::Uuid;

pub struct HardDeleteCvService<A, R>
where
    A: CVArchiver + Send + Sync,
    R: CVRepository + Send + Sync,
{
    cv_archiver: A,
    cv_repository: R,
}

impl<A, R> HardDeleteCvService<A, R>
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
impl<A, R> HardDeleteCvUseCase for HardDeleteCvService<A, R>
where
    A: CVArchiver + Send + Sync,
    R: CVRepository + Send + Sync,
{
    async fn execute(&self, user_id: UserId, cv_id: Uuid) -> Result<(), HardDeleteCVError> {
        // First, verify CV exists and belongs to the user
        let cv = self
            .cv_repository
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|e| match e {
                CVRepositoryError::NotFound => HardDeleteCVError::CVNotFound,
                CVRepositoryError::DatabaseError(msg) => HardDeleteCVError::RepositoryError(msg),
            })?
            .ok_or(HardDeleteCVError::CVNotFound)?;

        // Check ownership
        if cv.user_id != user_id.value() {
            return Err(HardDeleteCVError::Unauthorized);
        }

        // Perform hard delete
        self.cv_archiver
            .hard_delete(cv_id)
            .await
            .map_err(|e| match e {
                CVArchiverError::NotFound => HardDeleteCVError::CVNotFound,
                CVArchiverError::AlreadyArchived => {
                    HardDeleteCVError::RepositoryError("CV is archived".to_string())
                }
                CVArchiverError::NotArchived => {
                    HardDeleteCVError::RepositoryError("CV is not archived".to_string())
                }
                CVArchiverError::DatabaseError(msg) => HardDeleteCVError::RepositoryError(msg),
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{
        CVArchiver, CVArchiverError, CVRepository, CVRepositoryError, CreateCVData, UpdateCVData,
    };
    use crate::cv::domain::entities::CVInfo;
    use async_trait::async_trait;
    use uuid::Uuid;

    struct MockCVArchiver {
        result: Result<(), CVArchiverError>,
    }

    #[async_trait]
    impl CVArchiver for MockCVArchiver {
        async fn soft_delete(&self, _cv_id: Uuid) -> Result<(), CVArchiverError> {
            unimplemented!()
        }

        async fn hard_delete(&self, _cv_id: Uuid) -> Result<(), CVArchiverError> {
            self.result.clone()
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

    fn create_cv_info(cv_id: Uuid, user_id: Uuid) -> CVInfo {
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

    #[tokio::test]
    async fn test_execute_success() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Ok(Some(create_cv_info(cv_id, user_id))),
        };

        let mock_archiver = MockCVArchiver { result: Ok(()) };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_cv_not_found() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository { result: Ok(None) };

        let mock_archiver = MockCVArchiver { result: Ok(()) };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HardDeleteCVError::CVNotFound));
    }

    #[tokio::test]
    async fn test_execute_unauthorized() {
        let cv_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Ok(Some(create_cv_info(cv_id, owner_id))),
        };

        let mock_archiver = MockCVArchiver { result: Ok(()) };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(other_user_id), cv_id).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HardDeleteCVError::Unauthorized
        ));
    }

    #[tokio::test]
    async fn test_execute_repository_not_found_error() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Err(CVRepositoryError::NotFound),
        };

        let mock_archiver = MockCVArchiver { result: Ok(()) };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HardDeleteCVError::CVNotFound));
    }

    #[tokio::test]
    async fn test_execute_repository_database_error() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Err(CVRepositoryError::DatabaseError(
                "Connection failed".to_string(),
            )),
        };

        let mock_archiver = MockCVArchiver { result: Ok(()) };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HardDeleteCVError::RepositoryError(msg) => {
                assert!(msg.contains("Connection failed"));
            }
            _ => panic!("Expected RepositoryError variant"),
        }
    }

    #[tokio::test]
    async fn test_execute_archiver_not_found_error() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Ok(Some(create_cv_info(cv_id, user_id))),
        };

        let mock_archiver = MockCVArchiver {
            result: Err(CVArchiverError::NotFound),
        };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HardDeleteCVError::CVNotFound));
    }

    #[tokio::test]
    async fn test_execute_archiver_already_archived_error() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Ok(Some(create_cv_info(cv_id, user_id))),
        };

        let mock_archiver = MockCVArchiver {
            result: Err(CVArchiverError::AlreadyArchived),
        };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HardDeleteCVError::RepositoryError(msg) => {
                assert!(msg.contains("archived"));
            }
            _ => panic!("Expected RepositoryError variant"),
        }
    }

    #[tokio::test]
    async fn test_execute_archiver_not_archived_error() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Ok(Some(create_cv_info(cv_id, user_id))),
        };

        let mock_archiver = MockCVArchiver {
            result: Err(CVArchiverError::NotArchived),
        };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HardDeleteCVError::RepositoryError(msg) => {
                assert!(msg.contains("not archived"));
            }
            _ => panic!("Expected RepositoryError variant"),
        }
    }

    #[tokio::test]
    async fn test_execute_archiver_database_error() {
        let cv_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let mock_repository = MockCVRepository {
            result: Ok(Some(create_cv_info(cv_id, user_id))),
        };

        let mock_archiver = MockCVArchiver {
            result: Err(CVArchiverError::DatabaseError("Delete failed".to_string())),
        };

        let service = HardDeleteCvService::new(mock_archiver, mock_repository);

        let result = service.execute(UserId::from(user_id), cv_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HardDeleteCVError::RepositoryError(msg) => {
                assert!(msg.contains("Delete failed"));
            }
            _ => panic!("Expected RepositoryError variant"),
        }
    }
}

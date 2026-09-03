//! Implements the snapshot use cases.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::cv::application::ports::outgoing::{CvSnapshot, CvSnapshotStore};
use crate::cv::application::use_cases::cv_snapshots::{
    CreateCvSnapshotUseCase, CvSnapshotError, GetCvSnapshotUseCase,
};

/// Implements the corresponding use-case contract.
pub struct CreateCvSnapshotService<S> {
    store: S,
}

impl<S> CreateCvSnapshotService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> CreateCvSnapshotUseCase for CreateCvSnapshotService<S>
where
    S: CvSnapshotStore + Send + Sync,
{
    async fn execute(&self, owner: UserId, cv_id: Uuid) -> Result<CvSnapshot, CvSnapshotError> {
        // Deliberately not idempotent. Two applications sent a week apart
        // should each carry what was actually sent, even if the CV did not
        // change in between — the second snapshot being identical is a fact
        // about the CV, not a reason to share a row.
        Ok(self.store.create(owner.value(), cv_id).await?)
    }
}

/// Implements the corresponding use-case contract.
pub struct GetCvSnapshotService<S> {
    store: S,
}

impl<S> GetCvSnapshotService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> GetCvSnapshotUseCase for GetCvSnapshotService<S>
where
    S: CvSnapshotStore + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        snapshot_id: Uuid,
    ) -> Result<CvSnapshot, CvSnapshotError> {
        self.store
            .find(owner.value(), snapshot_id)
            .await?
            .ok_or(CvSnapshotError::SnapshotNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::CvSnapshotStoreError;
    use crate::cv::domain::entities::CVInfo;
    use chrono::Utc;
    use std::sync::Mutex;

    fn a_cv(id: Uuid, role: &str) -> CVInfo {
        CVInfo {
            id,
            user_id: Uuid::new_v4(),
            role: role.to_string(),
            display_name: "Jane Doe".into(),
            bio: "Backend engineer".into(),
            photo_url: String::new(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    /// Stores what it was handed and never looks at the CV again — which is
    /// the property being tested, not an implementation shortcut.
    #[derive(Default)]
    struct FakeStore {
        rows: Mutex<Vec<CvSnapshot>>,
        missing_cv: bool,
        /// What the living CV says right now. Snapshots must not follow it.
        current_role: Mutex<String>,
    }

    #[async_trait]
    impl CvSnapshotStore for FakeStore {
        async fn create(
            &self,
            owner: Uuid,
            cv_id: Uuid,
        ) -> Result<CvSnapshot, CvSnapshotStoreError> {
            if self.missing_cv {
                return Err(CvSnapshotStoreError::CvNotFound);
            }
            let snapshot = CvSnapshot {
                id: Uuid::new_v4(),
                cv_id,
                user_id: owner,
                document: a_cv(cv_id, &self.current_role.lock().unwrap()),
                created_at: Utc::now(),
            };
            self.rows.lock().unwrap().push(snapshot.clone());
            Ok(snapshot)
        }

        async fn find(
            &self,
            _owner: Uuid,
            snapshot_id: Uuid,
        ) -> Result<Option<CvSnapshot>, CvSnapshotStoreError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.id == snapshot_id)
                .cloned())
        }
    }

    /// So the tests can hold the fake and hand it to the service at once.
    #[async_trait]
    impl CvSnapshotStore for std::sync::Arc<FakeStore> {
        async fn create(&self, o: Uuid, c: Uuid) -> Result<CvSnapshot, CvSnapshotStoreError> {
            (**self).create(o, c).await
        }
        async fn find(&self, o: Uuid, s: Uuid) -> Result<Option<CvSnapshot>, CvSnapshotStoreError> {
            (**self).find(o, s).await
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    /// The whole reason snapshots exist: editing the CV afterwards must not
    /// change what a past application says was sent.
    #[tokio::test]
    async fn a_snapshot_does_not_follow_the_cv_it_came_from() {
        let store = std::sync::Arc::new(FakeStore {
            current_role: Mutex::new("Backend Engineer".into()),
            ..Default::default()
        });
        let cv_id = Uuid::new_v4();

        let taken = CreateCvSnapshotService::new(store.clone())
            .execute(owner(), cv_id)
            .await
            .unwrap();

        // The author keeps working on the CV after applying.
        *store.current_role.lock().unwrap() = "Staff Engineer".into();

        let read_back = GetCvSnapshotService::new(store)
            .execute(owner(), taken.id)
            .await
            .unwrap();

        assert_eq!(
            read_back.document.role, "Backend Engineer",
            "the snapshot must report what was sent, not what the CV says now"
        );
    }

    /// Not idempotent on purpose — two applications a week apart each carry
    /// what was actually sent, even when the CV did not change between them.
    #[tokio::test]
    async fn taking_two_snapshots_gives_two_rows() {
        let store = std::sync::Arc::new(FakeStore::default());
        let service = CreateCvSnapshotService::new(store.clone());
        let cv_id = Uuid::new_v4();

        let first = service.execute(owner(), cv_id).await.unwrap();
        let second = service.execute(owner(), cv_id).await.unwrap();

        assert_ne!(first.id, second.id);
        assert_eq!(store.rows.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn snapshotting_another_users_cv_is_not_found() {
        let store = FakeStore {
            missing_cv: true,
            ..Default::default()
        };

        let err = CreateCvSnapshotService::new(store)
            .execute(owner(), Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(matches!(err, CvSnapshotError::CvNotFound));
    }

    #[tokio::test]
    async fn an_unknown_snapshot_is_not_found() {
        let err = GetCvSnapshotService::new(FakeStore::default())
            .execute(owner(), Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(matches!(err, CvSnapshotError::SnapshotNotFound));
    }
}

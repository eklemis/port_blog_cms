//! Starting, listing, editing and archiving applications.
//!
//! The one piece of policy here is the snapshot rule: an application may not
//! leave draft without a frozen CV to point at.

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::incoming::use_cases::{
    ApplicationError, ArchiveApplicationUseCase, CreateApplicationUseCase, GetApplicationUseCase,
    GetApplicationsUseCase, PatchApplicationUseCase, UpdateApplicationInput,
};
use crate::career::application::ports::outgoing::{
    ApplicationStore, CreateApplicationData, CvSnapshotter, PatchApplicationData,
};
use crate::career::domain::entities::Application;

/// Implements the corresponding use-case contract.
pub struct ApplicationService<S> {
    store: S,
    snapshots: Arc<dyn CvSnapshotter>,
}

impl<S> ApplicationService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(store: S, snapshots: Arc<dyn CvSnapshotter>) -> Self {
        Self { store, snapshots }
    }
}

#[async_trait]
impl<S> CreateApplicationUseCase for ApplicationService<S>
where
    S: ApplicationStore + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        data: CreateApplicationData,
    ) -> Result<Application, ApplicationError> {
        // Always a draft. Sending is an edit, and that edit is where the
        // snapshot rule applies — so there is exactly one path that can
        // produce a sent application, rather than two to keep in step.
        Ok(self.store.create(owner.value(), data).await?)
    }
}

#[async_trait]
impl<S> GetApplicationsUseCase for ApplicationService<S>
where
    S: ApplicationStore + Send + Sync,
{
    async fn execute(&self, owner: UserId) -> Result<Vec<Application>, ApplicationError> {
        Ok(self.store.list(owner.value()).await?)
    }
}

#[async_trait]
impl<S> GetApplicationUseCase for ApplicationService<S>
where
    S: ApplicationStore + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        application_id: Uuid,
    ) -> Result<Application, ApplicationError> {
        self.store
            .find(owner.value(), application_id)
            .await?
            .ok_or(ApplicationError::NotFound)
    }
}

#[async_trait]
impl<S> PatchApplicationUseCase for ApplicationService<S>
where
    S: ApplicationStore + Send + Sync,
{
    async fn execute(
        &self,
        owner: UserId,
        application_id: Uuid,
        input: UpdateApplicationInput,
    ) -> Result<Application, ApplicationError> {
        let current = self
            .store
            .find(owner.value(), application_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        let mut patch = PatchApplicationData {
            status: input.status,
            next_action: input.next_action,
            next_action_at: input.next_action_at,
            ..Default::default()
        };

        // A CV was named: freeze it, whatever else this edit does. Naming a CV
        // is itself the instruction to take a copy.
        if let Some(cv_id) = input.cv_id {
            patch.cv_snapshot_id = Some(self.snapshots.snapshot(owner.value(), cv_id).await?);
        }

        let leaving_draft =
            current.status.is_draft() && input.status.map(|s| !s.is_draft()).unwrap_or(false);

        if leaving_draft {
            // The rule this service exists to enforce. A sent application with
            // no snapshot points at a living CV, and every later reading of it
            // is wrong in a way nobody notices until an interview.
            if patch.cv_snapshot_id.is_none() && current.cv_snapshot_id.is_none() {
                return Err(ApplicationError::SnapshotRequired);
            }

            // Stamp when it was sent, unless the row already carries one — a
            // reopened application that is sent again keeps its first date,
            // because that is the date the employer saw.
            if current.applied_at.is_none() {
                patch.applied_at = Some(Utc::now());
            }
        }

        if patch.is_empty() {
            return Ok(current);
        }

        Ok(self
            .store
            .patch(owner.value(), application_id, patch)
            .await?)
    }
}

#[async_trait]
impl<S> ArchiveApplicationUseCase for ApplicationService<S>
where
    S: ApplicationStore + Send + Sync,
{
    async fn execute(&self, owner: UserId, application_id: Uuid) -> Result<(), ApplicationError> {
        Ok(self.store.archive(owner.value(), application_id).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::application::ports::outgoing::{ApplicationStoreError, CvSnapshotterError};
    use crate::career::domain::entities::ApplicationStatus;
    use std::sync::Mutex;

    fn an_application(status: ApplicationStatus, snapshot: Option<Uuid>) -> Application {
        Application {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            cv_snapshot_id: snapshot,
            status,
            applied_at: None,
            next_action: String::new(),
            next_action_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[derive(Default)]
    struct FakeStore {
        row: Mutex<Option<Application>>,
        patches: Mutex<Vec<PatchApplicationData>>,
    }

    #[async_trait]
    impl ApplicationStore for Arc<FakeStore> {
        async fn create(
            &self,
            _o: Uuid,
            _d: CreateApplicationData,
        ) -> Result<Application, ApplicationStoreError> {
            unimplemented!()
        }
        async fn list(&self, _o: Uuid) -> Result<Vec<Application>, ApplicationStoreError> {
            unimplemented!()
        }
        async fn find(
            &self,
            _o: Uuid,
            _id: Uuid,
        ) -> Result<Option<Application>, ApplicationStoreError> {
            Ok(self.row.lock().unwrap().clone())
        }
        async fn patch(
            &self,
            _o: Uuid,
            _id: Uuid,
            data: PatchApplicationData,
        ) -> Result<Application, ApplicationStoreError> {
            let mut row = self.row.lock().unwrap().clone().unwrap();
            if let Some(s) = data.status {
                row.status = s;
            }
            if let Some(s) = data.cv_snapshot_id {
                row.cv_snapshot_id = Some(s);
            }
            if let Some(t) = data.applied_at {
                row.applied_at = Some(t);
            }
            self.patches.lock().unwrap().push(data);
            Ok(row)
        }
        async fn archive(&self, _o: Uuid, _id: Uuid) -> Result<(), ApplicationStoreError> {
            unimplemented!()
        }
    }

    struct FakeSnapshotter {
        taken: Mutex<Vec<Uuid>>,
        missing_cv: bool,
    }

    #[async_trait]
    impl CvSnapshotter for FakeSnapshotter {
        async fn snapshot(&self, _o: Uuid, cv_id: Uuid) -> Result<Uuid, CvSnapshotterError> {
            if self.missing_cv {
                return Err(CvSnapshotterError::CvNotFound);
            }
            self.taken.lock().unwrap().push(cv_id);
            Ok(Uuid::new_v4())
        }
    }

    /// Returned as the trait the tests exercise: the service implements
    /// several use-case traits and they all have an `execute`.
    fn service(
        store: Arc<FakeStore>,
        snapshots: Arc<FakeSnapshotter>,
    ) -> Arc<dyn PatchApplicationUseCase> {
        Arc::new(ApplicationService::new(store, snapshots))
    }

    fn snapshotter() -> Arc<FakeSnapshotter> {
        Arc::new(FakeSnapshotter {
            taken: Mutex::new(vec![]),
            missing_cv: false,
        })
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    // ------------------------------------------------------------------
    // The snapshot rule
    // ------------------------------------------------------------------

    /// The rule this service exists to enforce. A sent application with no
    /// snapshot points at a living CV, and every later reading of it is wrong
    /// in a way nobody notices until an interview.
    #[tokio::test]
    async fn leaving_draft_without_a_cv_is_refused() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(an_application(ApplicationStatus::Draft, None));

        let err = service(Arc::clone(&store), snapshotter())
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    status: Some(ApplicationStatus::Applied),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, ApplicationError::SnapshotRequired));
        assert!(
            store.patches.lock().unwrap().is_empty(),
            "nothing may be written when the rule refuses"
        );
    }

    #[tokio::test]
    async fn naming_a_cv_takes_the_snapshot_and_lets_it_through() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(an_application(ApplicationStatus::Draft, None));
        let snaps = snapshotter();
        let cv_id = Uuid::new_v4();

        let stored = service(Arc::clone(&store), Arc::clone(&snaps))
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    status: Some(ApplicationStatus::Applied),
                    cv_id: Some(cv_id),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(snaps.taken.lock().unwrap().as_slice(), &[cv_id]);
        assert!(stored.cv_snapshot_id.is_some());
        assert_eq!(stored.status, ApplicationStatus::Applied);
    }

    /// A snapshot attached by an earlier edit is enough — the rule is about
    /// having one, not about taking one on this particular call.
    #[tokio::test]
    async fn an_existing_snapshot_satisfies_the_rule() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(an_application(
            ApplicationStatus::Draft,
            Some(Uuid::new_v4()),
        ));
        let snaps = snapshotter();

        let stored = service(store, Arc::clone(&snaps))
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    status: Some(ApplicationStatus::Applied),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(stored.status, ApplicationStatus::Applied);
        assert!(
            snaps.taken.lock().unwrap().is_empty(),
            "no second snapshot should be taken"
        );
    }

    /// The rule applies to leaving draft, not to every edit. Renaming the next
    /// action on a draft must not demand a CV.
    #[tokio::test]
    async fn editing_a_draft_without_sending_it_needs_no_cv() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(an_application(ApplicationStatus::Draft, None));

        let result = service(store, snapshotter())
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    next_action: Some("Chase the recruiter".into()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_ok());
    }

    /// Moving between sent statuses is not "leaving draft" and must not be
    /// blocked — by then the snapshot already exists.
    #[tokio::test]
    async fn advancing_an_already_sent_application_is_unaffected() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(an_application(
            ApplicationStatus::Applied,
            Some(Uuid::new_v4()),
        ));

        let stored = service(store, snapshotter())
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    status: Some(ApplicationStatus::Interview),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(stored.status, ApplicationStatus::Interview);
    }

    #[tokio::test]
    async fn naming_a_cv_that_is_not_yours_is_cv_not_found() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(an_application(ApplicationStatus::Draft, None));
        let snaps = Arc::new(FakeSnapshotter {
            taken: Mutex::new(vec![]),
            missing_cv: true,
        });

        let err = service(store, snaps)
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    status: Some(ApplicationStatus::Applied),
                    cv_id: Some(Uuid::new_v4()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, ApplicationError::CvNotFound));
    }

    // ------------------------------------------------------------------
    // applied_at
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn sending_stamps_the_date() {
        let store = Arc::new(FakeStore::default());
        *store.row.lock().unwrap() = Some(an_application(
            ApplicationStatus::Draft,
            Some(Uuid::new_v4()),
        ));

        let stored = service(Arc::clone(&store), snapshotter())
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    status: Some(ApplicationStatus::Applied),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(stored.applied_at.is_some());
    }

    /// A reopened application that is sent again keeps its first date — that
    /// is the date the employer saw.
    #[tokio::test]
    async fn resending_does_not_move_the_original_date() {
        let store = Arc::new(FakeStore::default());
        let first_sent = Utc::now() - chrono::Duration::days(30);
        let mut row = an_application(ApplicationStatus::Draft, Some(Uuid::new_v4()));
        row.applied_at = Some(first_sent);
        *store.row.lock().unwrap() = Some(row);

        service(Arc::clone(&store), snapshotter())
            .execute(
                owner(),
                Uuid::new_v4(),
                UpdateApplicationInput {
                    status: Some(ApplicationStatus::Applied),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let patch = &store.patches.lock().unwrap()[0];
        assert!(
            patch.applied_at.is_none(),
            "the original applied_at must not be overwritten"
        );
    }
}

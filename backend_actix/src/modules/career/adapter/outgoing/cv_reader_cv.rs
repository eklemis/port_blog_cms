//! Supplies `career` with CVs to analyse, without `career` learning how CVs
//! are stored.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::career::application::ports::outgoing::{CvReader, CvReaderError};
use crate::cv::application::ports::outgoing::{CVQuery, CvSnapshotStore};
use crate::cv::domain::entities::CVInfo;

/// Reads CVs through the `cv` module.
pub struct CvReaderCv<Q> {
    query: Q,
    snapshots: Arc<dyn CvSnapshotStore>,
}

impl<Q> CvReaderCv<Q> {
    /// Builds it from the ports it depends on.
    pub fn new(query: Q, snapshots: Arc<dyn CvSnapshotStore>) -> Self {
        Self { query, snapshots }
    }
}

#[async_trait]
impl<Q> CvReader for CvReaderCv<Q>
where
    Q: CVQuery + Send + Sync,
{
    async fn read_cv(&self, owner: Uuid, cv_id: Uuid) -> Result<Option<CVInfo>, CvReaderError> {
        let cv = self
            .query
            .fetch_cv_by_id(cv_id)
            .await
            .map_err(|e| CvReaderError::Failed(e.to_string()))?;

        // `fetch_cv_by_id` is **not owner-scoped** — it takes an id and
        // nothing else, because its other caller is the public CV read where
        // the owner is resolved separately. The check therefore has to happen
        // here, or naming someone else's cv_id would analyse their CV and
        // report its contents back through the checks' `detail` strings.
        Ok(cv.filter(|cv| cv.user_id == owner))
    }

    async fn read_snapshot(
        &self,
        owner: Uuid,
        snapshot_id: Uuid,
    ) -> Result<Option<CVInfo>, CvReaderError> {
        // Owner-scoped in SQL by the snapshot store itself.
        self.snapshots
            .find(owner, snapshot_id)
            .await
            .map(|s| s.map(|s| s.document))
            .map_err(|e| CvReaderError::Failed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::application::ports::outgoing::{CVQueryError, CvSnapshot, CvSnapshotStoreError};

    fn a_cv(user_id: Uuid) -> CVInfo {
        CVInfo {
            id: Uuid::new_v4(),
            user_id,
            role: "Engineer".into(),
            display_name: "Jane".into(),
            bio: String::new(),
            photo_url: String::new(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    struct StubQuery(Option<CVInfo>);

    #[async_trait]
    impl CVQuery for StubQuery {
        async fn list(
            &self,
            _user_id: Uuid,
            _filter: crate::cv::application::ports::outgoing::CVListFilter,
            _sort: crate::cv::application::ports::outgoing::CVSort,
            _page: crate::cv::application::ports::outgoing::CVPageRequest,
        ) -> Result<crate::cv::application::ports::outgoing::CVPageResult<CVInfo>, CVQueryError>
        {
            unimplemented!()
        }
        async fn fetch_cv_by_id(&self, _id: Uuid) -> Result<Option<CVInfo>, CVQueryError> {
            Ok(self.0.clone())
        }
    }

    struct StubSnapshots;

    #[async_trait]
    impl CvSnapshotStore for StubSnapshots {
        async fn create(&self, _o: Uuid, _c: Uuid) -> Result<CvSnapshot, CvSnapshotStoreError> {
            unimplemented!()
        }
        async fn find(
            &self,
            _o: Uuid,
            _s: Uuid,
        ) -> Result<Option<CvSnapshot>, CvSnapshotStoreError> {
            Ok(None)
        }
    }

    fn reader(cv: Option<CVInfo>) -> CvReaderCv<StubQuery> {
        CvReaderCv::new(StubQuery(cv), Arc::new(StubSnapshots))
    }

    #[tokio::test]
    async fn a_cv_the_caller_owns_is_returned() {
        let owner = Uuid::new_v4();

        let result = reader(Some(a_cv(owner)))
            .read_cv(owner, Uuid::new_v4())
            .await;

        assert!(result.unwrap().is_some());
    }

    /// The underlying query takes an id and nothing else, so without this
    /// filter a caller could analyse anyone's CV by naming its id — and read
    /// its contents back out of the checks' detail strings.
    #[tokio::test]
    async fn another_users_cv_reads_as_absent() {
        let someone_else = Uuid::new_v4();

        let result = reader(Some(a_cv(someone_else)))
            .read_cv(Uuid::new_v4(), Uuid::new_v4())
            .await;

        assert!(
            result.unwrap().is_none(),
            "a CV belonging to someone else must not be readable here"
        );
    }
}

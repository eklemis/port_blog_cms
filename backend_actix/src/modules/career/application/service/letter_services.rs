//! Cover letters and reflections.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::career::application::ports::incoming::use_cases::{
    CoverLetterUseCases, LetterError, ReflectionUseCases,
};
use crate::career::application::ports::outgoing::{
    LetterStore, PatchCoverLetterData, ReflectionData,
};
use crate::career::domain::entities::{CoverLetter, Reflection};

/// Implements both letter contracts over one store.
pub struct LetterService<S> {
    store: S,
}

impl<S> LetterService<S> {
    /// Builds it from the ports it depends on.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> CoverLetterUseCases for LetterService<S>
where
    S: LetterStore + Send + Sync,
{
    async fn get(&self, owner: UserId, application_id: Uuid) -> Result<CoverLetter, LetterError> {
        self.store
            .find_letter(owner.value(), application_id)
            .await?
            .ok_or(LetterError::NotFound)
    }

    async fn write(
        &self,
        owner: UserId,
        application_id: Uuid,
        data: PatchCoverLetterData,
    ) -> Result<CoverLetter, LetterError> {
        // An empty patch is a read: writing would bump updated_at and report
        // success for having done nothing.
        if data.is_empty() {
            return CoverLetterUseCases::get(self, owner, application_id).await;
        }

        Ok(self
            .store
            .upsert_letter(owner.value(), application_id, data)
            .await?)
    }

    async fn delete(&self, owner: UserId, application_id: Uuid) -> Result<(), LetterError> {
        Ok(self
            .store
            .delete_letter(owner.value(), application_id)
            .await?)
    }
}

#[async_trait]
impl<S> ReflectionUseCases for LetterService<S>
where
    S: LetterStore + Send + Sync,
{
    async fn get(&self, owner: UserId, application_id: Uuid) -> Result<Reflection, LetterError> {
        self.store
            .find_reflection(owner.value(), application_id)
            .await?
            .ok_or(LetterError::NotFound)
    }

    async fn write(
        &self,
        owner: UserId,
        application_id: Uuid,
        data: ReflectionData,
    ) -> Result<Reflection, LetterError> {
        // Written whole rather than patched. The three questions are answered
        // in one sitting, and a partial update would let a half-finished
        // thought overwrite a finished one field by field.
        Ok(self
            .store
            .upsert_reflection(owner.value(), application_id, data)
            .await?)
    }

    async fn delete(&self, owner: UserId, application_id: Uuid) -> Result<(), LetterError> {
        Ok(self
            .store
            .delete_reflection(owner.value(), application_id)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::application::ports::outgoing::LetterStoreError;
    use crate::career::domain::entities::CoverLetterStatus;
    use chrono::Utc;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeStore {
        letter: Mutex<Option<CoverLetter>>,
        reflection: Mutex<Option<Reflection>>,
        writes: Mutex<Vec<PatchCoverLetterData>>,
    }

    #[async_trait]
    impl LetterStore for Arc<FakeStore> {
        async fn find_letter(
            &self,
            _o: Uuid,
            _id: Uuid,
        ) -> Result<Option<CoverLetter>, LetterStoreError> {
            Ok(self.letter.lock().unwrap().clone())
        }

        async fn upsert_letter(
            &self,
            _o: Uuid,
            application_id: Uuid,
            data: PatchCoverLetterData,
        ) -> Result<CoverLetter, LetterStoreError> {
            let mut current = self.letter.lock().unwrap().clone().unwrap_or(CoverLetter {
                application_id,
                content: String::new(),
                language: "en".into(),
                status: CoverLetterStatus::Draft,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
            if let Some(c) = data.content.clone() {
                current.content = c;
            }
            if let Some(l) = data.language.clone() {
                current.language = l;
            }
            if let Some(s) = data.status {
                current.status = s;
            }
            self.writes.lock().unwrap().push(data);
            *self.letter.lock().unwrap() = Some(current.clone());
            Ok(current)
        }

        async fn delete_letter(&self, _o: Uuid, _id: Uuid) -> Result<(), LetterStoreError> {
            *self.letter.lock().unwrap() = None;
            Ok(())
        }

        async fn find_reflection(
            &self,
            _o: Uuid,
            _id: Uuid,
        ) -> Result<Option<Reflection>, LetterStoreError> {
            Ok(self.reflection.lock().unwrap().clone())
        }

        async fn upsert_reflection(
            &self,
            _o: Uuid,
            application_id: Uuid,
            data: ReflectionData,
        ) -> Result<Reflection, LetterStoreError> {
            let r = Reflection {
                application_id,
                stage_reached: data.stage_reached,
                what_happened: data.what_happened,
                what_id_change: data.what_id_change,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            *self.reflection.lock().unwrap() = Some(r.clone());
            Ok(r)
        }

        async fn delete_reflection(&self, _o: Uuid, _id: Uuid) -> Result<(), LetterStoreError> {
            *self.reflection.lock().unwrap() = None;
            Ok(())
        }
    }

    fn owner() -> UserId {
        UserId::from(Uuid::new_v4())
    }

    fn letters(store: Arc<FakeStore>) -> Arc<dyn CoverLetterUseCases> {
        Arc::new(LetterService::new(store))
    }

    fn reflections(store: Arc<FakeStore>) -> Arc<dyn ReflectionUseCases> {
        Arc::new(LetterService::new(store))
    }

    // ── cover letters ──────────────────────────────────────────────────

    /// Same semantics as the blog editor: an omitted field keeps what is
    /// stored, so a partial save cannot blank the body.
    #[tokio::test]
    async fn writing_only_the_status_leaves_the_body_alone() {
        let store = Arc::new(FakeStore::default());
        letters(Arc::clone(&store))
            .write(
                owner(),
                Uuid::new_v4(),
                PatchCoverLetterData {
                    content: Some("Dear hiring manager".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let stored = letters(Arc::clone(&store))
            .write(
                owner(),
                Uuid::new_v4(),
                PatchCoverLetterData {
                    status: Some(CoverLetterStatus::Sent),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(stored.content, "Dear hiring manager");
        assert_eq!(stored.status, CoverLetterStatus::Sent);
    }

    /// An empty patch reads rather than writes: writing would bump
    /// `updated_at` and report success for having done nothing.
    #[tokio::test]
    async fn an_empty_patch_does_not_write() {
        let store = Arc::new(FakeStore::default());
        *store.letter.lock().unwrap() = Some(CoverLetter {
            application_id: Uuid::new_v4(),
            content: "Draft".into(),
            language: "en".into(),
            status: CoverLetterStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        letters(Arc::clone(&store))
            .write(owner(), Uuid::new_v4(), PatchCoverLetterData::default())
            .await
            .unwrap();

        assert!(store.writes.lock().unwrap().is_empty());
    }

    /// The letter's language is its own, and is never inferred from the text.
    #[tokio::test]
    async fn a_letter_carries_its_own_language() {
        let store = Arc::new(FakeStore::default());

        let stored = letters(store)
            .write(
                owner(),
                Uuid::new_v4(),
                PatchCoverLetterData {
                    content: Some("Yang terhormat".into()),
                    language: Some("id".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(stored.language, "id");
    }

    // ── reflections ────────────────────────────────────────────────────

    /// Written whole, unlike the letter: the three answers are given in one
    /// sitting, and a partial update would let a half-finished thought
    /// overwrite a finished one field by field.
    #[tokio::test]
    async fn writing_a_reflection_replaces_all_three_answers() {
        let store = Arc::new(FakeStore::default());
        let id = Uuid::new_v4();

        reflections(Arc::clone(&store))
            .write(
                owner(),
                id,
                ReflectionData {
                    stage_reached: "Final".into(),
                    what_happened: "Take-home".into(),
                    what_id_change: "Practise".into(),
                },
            )
            .await
            .unwrap();

        let stored = reflections(Arc::clone(&store))
            .write(
                owner(),
                id,
                ReflectionData {
                    stage_reached: "Screening".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(stored.stage_reached, "Screening");
        assert_eq!(stored.what_happened, "");
    }

    /// Real deletion. Someone withdrawing a private note about their own
    /// rejection should not later discover it was only hidden.
    #[tokio::test]
    async fn deleting_a_reflection_really_removes_it() {
        let store = Arc::new(FakeStore::default());
        reflections(Arc::clone(&store))
            .write(owner(), Uuid::new_v4(), ReflectionData::default())
            .await
            .unwrap();

        reflections(Arc::clone(&store))
            .delete(owner(), Uuid::new_v4())
            .await
            .unwrap();

        assert!(store.reflection.lock().unwrap().is_none());
        assert!(matches!(
            reflections(store).get(owner(), Uuid::new_v4()).await,
            Err(LetterError::NotFound)
        ));
    }
}

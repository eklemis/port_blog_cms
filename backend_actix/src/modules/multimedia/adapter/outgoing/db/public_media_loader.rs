//! Loads public media projections for other modules' read paths.
//!
//! `blog` and `project` both need "the media attached to these rows, as public
//! URLs". Rather than each learning the media schema, they call in here — so
//! the join and the URL shape live once, in the module that owns the tables.

use std::collections::{BTreeMap, HashMap};

use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::multimedia::adapter::outgoing::db::sea_orm_entity::{media_attachments, media_variants};
use crate::multimedia::application::domain::entities::{AttachmentTarget, PublicMedia};

/// The public URL for one variant.
///
/// A path into this API, never a bucket URL: the bucket is private and
/// `GET /api/public/media/{id}/{size}` signs and redirects per fetch. That is
/// what lets a cached page hold this string indefinitely — see
/// `docs/adr/0006-public-media-urls.md`.
fn variant_url(media_id: Uuid, size: &str) -> String {
    format!("/api/public/media/{media_id}/{size}")
}

/// Media attached to each of `target_ids`, keyed by target id.
///
/// Two queries rather than a join: attachments, then the variants for the media
/// those attachments name. A join would multiply every attachment by its
/// variant count and need de-duplicating in Rust anyway.
///
/// Batched over ids on purpose — a listing page calls this once for the whole
/// page rather than once per row.
///
/// Targets with no media are simply absent from the map. An attachment whose
/// variants have not been generated yet appears with an empty `variants` map
/// rather than being hidden: the image is coming, not missing.
pub async fn load_public_media(
    db: &DatabaseConnection,
    target: AttachmentTarget,
    target_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<PublicMedia>>, DbErr> {
    if target_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let attachments = media_attachments::Entity::find()
        .filter(media_attachments::Column::AttachableType.eq(target.to_string()))
        .filter(media_attachments::Column::AttachableId.is_in(target_ids.to_vec()))
        .order_by_asc(media_attachments::Column::Role)
        .order_by_asc(media_attachments::Column::Position)
        .all(db)
        .await?;

    if attachments.is_empty() {
        return Ok(HashMap::new());
    }

    let media_ids: Vec<Uuid> = attachments.iter().map(|a| a.media_id).collect();
    let variants = media_variants::Entity::find()
        .filter(media_variants::Column::MediaId.is_in(media_ids))
        .all(db)
        .await?;

    let mut by_media: HashMap<Uuid, BTreeMap<String, String>> = HashMap::new();
    for v in variants {
        by_media.entry(v.media_id).or_default().insert(
            v.variant_type.clone(),
            variant_url(v.media_id, &v.variant_type),
        );
    }

    let mut out: HashMap<Uuid, Vec<PublicMedia>> = HashMap::new();
    for a in attachments {
        out.entry(a.attachable_id).or_default().push(PublicMedia {
            media_id: a.media_id,
            alt_text: a.alt_text.unwrap_or_default(),
            caption: a.caption.unwrap_or_default(),
            role: a.role,
            position: a.position,
            variants: by_media.get(&a.media_id).cloned().unwrap_or_default(),
        });
    }
    Ok(out)
}

/// The media attached to exactly one target.
///
/// A thin wrapper over [`load_public_media`] for the single-item read paths.
pub async fn load_public_media_for(
    db: &DatabaseConnection,
    target: AttachmentTarget,
    target_id: Uuid,
) -> Result<Vec<PublicMedia>, DbErr> {
    Ok(load_public_media(db, target, &[target_id])
        .await?
        .remove(&target_id)
        .unwrap_or_default())
}

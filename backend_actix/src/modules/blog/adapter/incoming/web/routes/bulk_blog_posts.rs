//! `POST /api/blog/bulk` — one operation applied to many posts.

use actix_web::{post, web, Responder};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::schemas::{ErrorResponse, SuccessResponse},
    auth::{
        adapter::incoming::web::extractors::auth::VerifiedUser,
        application::domain::entities::UserId,
    },
    blog::application::ports::incoming::use_cases::BlogBulkOp,
    shared::api::{ApiResponse, BulkOutcome},
    AppState,
};

/// Request body: the operation, then the posts to apply it to.
///
/// `op` and its arguments are flattened into this object, so an attach reads
/// `{"op": "attach_topic", "topic_id": "...", "ids": [...]}`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkBlogRequest {
    /// What to do. One of `archive`, `restore`, `hard_delete`,
    /// `attach_topic`, `detach_topic`; the topic operations also take
    /// `topic_id`.
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub op: BlogBulkOp,

    /// The posts to apply it to. Duplicates are collapsed.
    pub ids: Vec<Uuid>,
}

/// Apply one operation to many posts
///
/// A batch is many operations, not one: each post succeeds or fails on its own
/// and the response says which. **A 200 means the batch ran, not that every
/// post succeeded** — read `failed`.
///
/// Posts belonging to another author are reported as `POST_NOT_FOUND`, the same
/// as the single-item routes, so this cannot be used to discover or modify
/// anyone else's work.
#[utoipa::path(
    post,
    path = "/api/blog/bulk",
    tag = "blog",
    request_body = BulkBlogRequest,
    responses(
        (
            status = 200,
            description = "The batch ran. Check `failed` for items that did not.",
            body = inline(SuccessResponse<BulkOutcome>),
            example = json!({
                "success": true,
                "data": {
                    "succeeded": ["3f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d"],
                    "failed": [{
                        "id": "9a8b7c6d-5e4f-3a2b-1c0d-9e8f7a6b5c4d",
                        "code": "POST_NOT_FOUND",
                        "message": "Blog post not found"
                    }]
                }
            })
        ),
        (
            status = 400,
            description = "The id list was empty or longer than the cap",
            body = ErrorResponse,
            example = json!({
                "success": false,
                "error": { "code": "BULK_TOO_LARGE", "message": "A bulk request carries at most 100 ids, got 250" }
            })
        ),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
    ),
    security(("BearerAuth" = []))
)]
#[post("/api/blog/bulk")]
pub async fn bulk_blog_posts_handler(
    user: VerifiedUser,
    body: web::Json<BulkBlogRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let body = body.into_inner();

    match data
        .blog
        .bulk
        .execute(UserId::from(user.user_id), body.op, body.ids)
        .await
    {
        Ok(outcome) => ApiResponse::success(outcome),
        Err(e) => ApiResponse::bad_request(e.code(), &e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tagged shape is what a client gets wrong first, so the three forms
    /// it must send are pinned here rather than left to the OpenAPI example.
    #[test]
    fn a_lifecycle_request_needs_only_op_and_ids() {
        let req: BulkBlogRequest = serde_json::from_str(
            r#"{"op":"archive","ids":["3f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d"]}"#,
        )
        .unwrap();

        assert_eq!(req.op, BlogBulkOp::Archive);
        assert_eq!(req.ids.len(), 1);
    }

    #[test]
    fn a_topic_request_carries_the_topic_alongside_op() {
        let req: BulkBlogRequest = serde_json::from_str(
            r#"{"op":"attach_topic","topic_id":"8f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d","ids":[]}"#,
        )
        .unwrap();

        match req.op {
            BlogBulkOp::AttachTopic { topic_id } => {
                assert_eq!(topic_id.to_string(), "8f1b2c3d-4e5f-6a7b-8c9d-0e1f2a3b4c5d");
            }
            other => panic!("expected attach_topic, got {other:?}"),
        }
    }

    /// The reason `op` is a tagged enum rather than a string beside an optional
    /// `topic_id`: "attach, but no topic" is rejected at the door instead of
    /// halfway through a batch.
    #[test]
    fn attach_without_a_topic_is_not_a_valid_request() {
        let err = serde_json::from_str::<BulkBlogRequest>(r#"{"op":"attach_topic","ids":[]}"#);

        assert!(err.is_err(), "attach_topic must require topic_id");
    }

    #[test]
    fn an_unknown_op_is_rejected() {
        let err = serde_json::from_str::<BulkBlogRequest>(r#"{"op":"publish","ids":[]}"#);

        assert!(err.is_err(), "only the declared operations are accepted");
    }

    /// snake_case on the wire, matching every other enum the API takes.
    #[test]
    fn hard_delete_is_snake_case() {
        let req: BulkBlogRequest =
            serde_json::from_str(r#"{"op":"hard_delete","ids":[]}"#).unwrap();

        assert_eq!(req.op, BlogBulkOp::HardDelete);
    }

    /// C1: unpublish is a bulk op; publish deliberately is not.
    #[tokio::test]
    async fn unpublish_is_an_accepted_op() {
        let req: BulkBlogRequest = serde_json::from_str(r#"{"op":"unpublish","ids":[]}"#).unwrap();

        assert_eq!(req.op, BlogBulkOp::Unpublish);
    }

    /// Not an oversight. Publishing is per-post considered work, and "publish
    /// these" is ambiguous about whether it means now or at each post's
    /// scheduled time — so the operation does not exist rather than picking an
    /// answer for the author.
    #[tokio::test]
    async fn publish_is_deliberately_not_an_op() {
        let err = serde_json::from_str::<BulkBlogRequest>(r#"{"op":"publish","ids":[]}"#);

        assert!(err.is_err(), "bulk publish must stay unavailable");
    }
}

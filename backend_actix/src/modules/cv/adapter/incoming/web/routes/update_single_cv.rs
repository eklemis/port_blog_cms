use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::cv::application::ports::outgoing::UpdateCVData;
use crate::cv::application::use_cases::update_cv::UpdateCVError;
use crate::cv::domain::entities::{
    ContactDetail, ContactType, CoreSkill, Education, Experience, HighlightedProject,
};
use crate::shared::api::ApiResponse;
use crate::AppState;
use actix_web::{put, web, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EducationRequest {
    pub degree: String,
    pub institution: String,
    pub graduation_year: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExperienceRequest {
    pub company: String,
    pub position: String,
    pub location: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: String,
    pub tasks: Vec<String>,
    pub achievements: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HighlightedProjectRequest {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: String,
}

type ContactTypeRequest = ContactType;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContactDetailRequest {
    pub title: String,
    pub contact_type: ContactTypeRequest,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateCVRequest {
    pub bio: String,
    pub role: String,
    pub photo_url: String,
    pub display_name: String,
    pub core_skills: Vec<CoreSkill>,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
    pub contact_info: Vec<ContactDetailRequest>,
}

#[put("/api/cvs/{cv_id}")]
pub async fn update_cv_handler(
    user: VerifiedUser,
    path: web::Path<Uuid>,
    req: web::Json<UpdateCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let cv_id = path.into_inner();

    let cv_data = UpdateCVData {
        bio: req.bio.clone(),
        role: req.role.clone(),
        display_name: req.display_name.clone(),
        photo_url: req.photo_url.clone(),
        core_skills: req
            .core_skills
            .iter()
            .map(|e| CoreSkill {
                title: e.title.clone(),
                description: e.description.clone(),
            })
            .collect(),
        educations: req
            .educations
            .iter()
            .map(|e| Education {
                degree: e.degree.clone(),
                institution: e.institution.clone(),
                graduation_year: e.graduation_year,
            })
            .collect(),
        experiences: req
            .experiences
            .iter()
            .map(|exp| Experience {
                company: exp.company.clone(),
                position: exp.position.clone(),
                location: exp.location.clone(),
                start_date: exp.start_date.clone(),
                end_date: exp.end_date.clone(),
                description: exp.description.clone(),
                tasks: exp.tasks.clone(),
                achievements: exp.achievements.clone(),
            })
            .collect(),
        highlighted_projects: req
            .highlighted_projects
            .iter()
            .map(|hp| HighlightedProject {
                id: hp.id.clone(),
                title: hp.title.clone(),
                slug: hp.slug.clone(),
                short_description: hp.short_description.clone(),
            })
            .collect(),
        contact_info: req
            .contact_info
            .iter()
            .map(|cd| ContactDetail {
                title: cd.title.clone(),
                contact_type: cd.contact_type.clone(),
                content: cd.content.clone(),
            })
            .collect(),
    };

    match data
        .update_cv_use_case
        .execute(user.user_id, cv_id, cv_data)
        .await
    {
        Ok(updated) => ApiResponse::success(updated),
        Err(UpdateCVError::CVNotFound) => ApiResponse::not_found("CV_NOT_FOUND", "CV not found"),
        Err(UpdateCVError::RepositoryError(e)) => {
            error!("Repository error updating CV: {}", e);
            ApiResponse::internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::application::ports::outgoing::token_provider::TokenProvider,
        cv::{application::use_cases::update_cv::IUpdateCVUseCase, domain::CVInfo},
        tests::support::{
            app_state_builder::TestAppStateBuilder,
            auth_helper::test_helpers::create_test_jwt_service,
        },
    };

    use super::*;
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    /* --------------------------------------------------
     * Helpers
     * -------------------------------------------------- */

    fn base_update_request() -> UpdateCVRequest {
        UpdateCVRequest {
            display_name: "Jonathan Verguso".to_string(),
            role: "New role".to_string(),
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        }
    }

    impl CVInfo {
        fn from_update(cv_id: Uuid, user_id: Uuid, data: UpdateCVData) -> Self {
            Self {
                id: cv_id,
                user_id,
                display_name: data.display_name,
                role: data.role,
                bio: data.bio,
                photo_url: data.photo_url,
                core_skills: data.core_skills,
                educations: data.educations,
                experiences: data.experiences,
                highlighted_projects: data.highlighted_projects,
                contact_info: data.contact_info,
            }
        }
    }

    /* --------------------------------------------------
     * Mock Update CV Use Case
     * -------------------------------------------------- */

    #[derive(Clone)]
    struct MockUpdateCVUseCase {
        should_fail: Arc<Mutex<Option<UpdateCVError>>>,
        updated_cv: Arc<Mutex<Option<CVInfo>>>,
    }

    impl MockUpdateCVUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                updated_cv: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: UpdateCVError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self, cv: CVInfo) {
            *self.updated_cv.lock().await = Some(cv);
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl IUpdateCVUseCase for MockUpdateCVUseCase {
        async fn execute(
            &self,
            user_id: Uuid,
            cv_id: Uuid,
            cv_data: UpdateCVData,
        ) -> Result<CVInfo, UpdateCVError> {
            if let Some(err) = self.should_fail.lock().await.clone() {
                return Err(err);
            }

            if let Some(cv) = self.updated_cv.lock().await.clone() {
                return Ok(cv);
            }

            Ok(CVInfo::from_update(cv_id, user_id, cv_data))
        }
    }

    /* --------------------------------------------------
     * Tests
     * -------------------------------------------------- */

    #[actix_web::test]
    async fn test_update_cv_handler_success() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let updated_cv = CVInfo {
            id: cv_id,
            user_id,
            display_name: "Berto Fang".to_string(),
            bio: "Software Engineer with 5 years of experience".to_string(),
            role: "Software Engineer".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![],
            educations: vec![Education {
                degree: "Bachelor of Computer Science".to_string(),
                institution: "MIT".to_string(),
                graduation_year: 2018,
            }],
            experiences: vec![],
            highlighted_projects: vec![],
            contact_info: vec![],
        };

        let update_uc = Arc::new(MockUpdateCVUseCase::new());
        update_uc.set_success(updated_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc.clone())
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());
        let token = jwt_service.generate_access_token(user_id, true).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(update_cv_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(UpdateCVRequest {
                display_name: "Berto Fang".to_string(),
                bio: "Software Engineer with 5 years of experience".to_string(),
                role: "Software Engineer".to_string(),
                photo_url: "https://example.com/photo.jpg".to_string(),
                educations: vec![EducationRequest {
                    degree: "Bachelor of Computer Science".to_string(),
                    institution: "MIT".to_string(),
                    graduation_year: 2018,
                }],
                ..base_update_request()
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["bio"], updated_cv.bio);
        assert_eq!(body["data"]["role"], updated_cv.role);
        assert_eq!(body["data"]["photo_url"], updated_cv.photo_url);
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_update_cv_handler_with_full_data() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let updated_cv = CVInfo {
            id: cv_id,
            user_id,
            display_name: "Mapping Test".to_string(),
            role: "QA Engineer".to_string(),
            bio: "Testing specialist".to_string(),
            photo_url: "https://example.com/qa.jpg".to_string(),
            core_skills: vec![CoreSkill {
                title: "Testing".to_string(),
                description: "Quality assurance".to_string(),
            }],
            educations: vec![Education {
                degree: "B.A.".to_string(),
                institution: "Test University".to_string(),
                graduation_year: 2019,
            }],
            experiences: vec![Experience {
                company: "TestCo".to_string(),
                position: "QA Lead".to_string(),
                location: "Boston, MA".to_string(),
                start_date: "2019-06-01".to_string(),
                end_date: Some("2022-05-31".to_string()),
                description: "Led QA team".to_string(),
                tasks: vec!["Test planning".to_string(), "Automation".to_string()],
                achievements: vec!["Zero critical bugs in production".to_string()],
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "test-proj".to_string(),
                title: "Test Automation Framework".to_string(),
                slug: "test-automation".to_string(),
                short_description: "Automated testing solution".to_string(),
            }],
            contact_info: vec![ContactDetail {
                contact_type: ContactType::WebPage,
                title: "Portfolio".to_string(),
                content: "https://qa-portfolio.com".to_string(),
            }],
        };

        let update_uc = Arc::new(MockUpdateCVUseCase::new());
        update_uc.set_success(updated_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc.clone())
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());
        let token = jwt_service.generate_access_token(user_id, true).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(update_cv_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(UpdateCVRequest {
                display_name: "Mapping Test".to_string(),
                role: "QA Engineer".to_string(),
                bio: "Testing specialist".to_string(),
                photo_url: "https://example.com/qa.jpg".to_string(),
                core_skills: vec![CoreSkill {
                    title: "Testing".to_string(),
                    description: "Quality assurance".to_string(),
                }],
                educations: vec![EducationRequest {
                    degree: "B.A.".to_string(),
                    institution: "Test University".to_string(),
                    graduation_year: 2019,
                }],
                experiences: vec![ExperienceRequest {
                    company: "TestCo".to_string(),
                    position: "QA Lead".to_string(),
                    location: "Boston, MA".to_string(),
                    start_date: "2019-06-01".to_string(),
                    end_date: Some("2022-05-31".to_string()),
                    description: "Led QA team".to_string(),
                    tasks: vec!["Test planning".to_string(), "Automation".to_string()],
                    achievements: vec!["Zero critical bugs in production".to_string()],
                }],
                highlighted_projects: vec![HighlightedProjectRequest {
                    id: "test-proj".to_string(),
                    title: "Test Automation Framework".to_string(),
                    slug: "test-automation".to_string(),
                    short_description: "Automated testing solution".to_string(),
                }],
                contact_info: vec![ContactDetailRequest {
                    contact_type: ContactType::WebPage,
                    title: "Portfolio".to_string(),
                    content: "https://qa-portfolio.com".to_string(),
                }],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["core_skills"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"]["educations"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"]["experiences"].as_array().unwrap().len(), 1);
        assert_eq!(
            body["data"]["highlighted_projects"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(body["data"]["contact_info"].as_array().unwrap().len(), 1);
        assert!(body.get("error").is_none());
    }

    #[actix_web::test]
    async fn test_update_cv_handler_not_found() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = Arc::new(MockUpdateCVUseCase::new());
        update_uc.set_error(UpdateCVError::CVNotFound).await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc.clone())
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());
        let token = jwt_service.generate_access_token(user_id, true).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(update_cv_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(base_update_request())
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 404);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "CV_NOT_FOUND");
        assert_eq!(body["error"]["message"], "CV not found");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_update_cv_handler_repository_error() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = Arc::new(MockUpdateCVUseCase::new());
        update_uc
            .set_error(UpdateCVError::RepositoryError(
                "DB update failed".to_string(),
            ))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc.clone())
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());
        let token = jwt_service.generate_access_token(user_id, true).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(update_cv_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(base_update_request())
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(body["error"]["message"], "An unexpected error occurred");
        assert!(body.get("data").is_none());
    }

    #[actix_web::test]
    async fn test_update_cv_handler_missing_authorization() {
        let cv_id = Uuid::new_v4();

        let update_uc = Arc::new(MockUpdateCVUseCase::new());

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc.clone())
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(update_cv_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .set_json(base_update_request())
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_update_cv_handler_invalid_token() {
        let cv_id = Uuid::new_v4();

        let update_uc = Arc::new(MockUpdateCVUseCase::new());

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc.clone())
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(update_cv_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .set_json(base_update_request())
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_update_cv_handler_unverified_user() {
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = Arc::new(MockUpdateCVUseCase::new());

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc.clone())
            .build();

        let jwt_service = create_test_jwt_service();
        let token_provider: Arc<dyn TokenProvider + Send + Sync> = Arc::new(jwt_service.clone());
        let token = jwt_service.generate_access_token(user_id, false).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(token_provider))
                .service(update_cv_handler),
        )
        .await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(base_update_request())
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 403);
    }
}

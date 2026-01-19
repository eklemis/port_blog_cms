use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::cv::application::ports::outgoing::UpdateCVData;
use crate::cv::application::use_cases::update_cv::UpdateCVError;
use crate::cv::domain::entities::{CoreSkill, Education, Experience, HighlightedProject};
use crate::AppState;
use actix_web::{put, web, HttpResponse, Responder};
use uuid::Uuid;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct EducationRequest {
    pub degree: String,
    pub institution: String,
    pub graduation_year: i32,
}

#[derive(serde::Deserialize, serde::Serialize)]
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

#[derive(serde::Deserialize, serde::Serialize)]
pub struct HighlightedProjectRequest {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct UpdateCVRequest {
    pub bio: String,
    pub role: String,
    pub photo_url: String,
    pub core_skills: Vec<CoreSkill>,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
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
    };

    match data
        .update_cv_use_case
        .execute(user.user_id, cv_id, cv_data)
        .await
    {
        Ok(updated) => HttpResponse::Ok().json(updated),

        Err(UpdateCVError::CVNotFound) => HttpResponse::NotFound().body("CV not found"),

        Err(UpdateCVError::RepositoryError(e)) => HttpResponse::InternalServerError().body(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
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

    // Mock Update CV Use Case
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
            _user_id: Uuid,
            cv_id: Uuid,
            cv_data: UpdateCVData,
        ) -> Result<CVInfo, UpdateCVError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }

            let cv = self.updated_cv.lock().await;
            if let Some(c) = cv.as_ref() {
                return Ok(c.clone());
            }

            // Convert UpdateCVData to CVInfo, using the provided cv_id
            Ok(CVInfo {
                id: cv_id,
                user_id: Uuid::new_v4(),
                role: cv_data.role,
                bio: cv_data.bio,
                photo_url: cv_data.photo_url,
                core_skills: cv_data.core_skills,
                educations: cv_data.educations,
                experiences: cv_data.experiences,
                highlighted_projects: cv_data.highlighted_projects,
            })
        }
    }

    #[actix_web::test]
    async fn test_update_cv_handler_success() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = MockUpdateCVUseCase::new();

        let updated_cv = CVInfo {
            id: cv_id,
            user_id,
            bio: "New bio".to_string(),
            role: "New role".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        update_uc.set_success(updated_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(update_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(UpdateCVRequest {
                bio: "New bio".to_string(),
                role: "New role".to_string(),
                photo_url: "https://example.com/new.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.bio, updated_cv.bio);
        assert_eq!(body.role, updated_cv.role);
        assert_eq!(body.photo_url, updated_cv.photo_url);
    }

    #[actix_web::test]
    async fn test_update_cv_handler_with_full_data() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = MockUpdateCVUseCase::new();

        let updated_cv = CVInfo {
            id: cv_id,
            user_id,
            bio: "Updated comprehensive bio".to_string(),
            role: "Lead Software Architect".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            core_skills: vec![CoreSkill {
                title: "System Design".to_string(),
                description: "Expert in distributed systems".to_string(),
            }],
            educations: vec![Education {
                degree: "Master of Computer Science".to_string(),
                institution: "Stanford".to_string(),
                graduation_year: 2020,
            }],
            experiences: vec![Experience {
                company: "Big Tech Inc".to_string(),
                position: "Tech Lead".to_string(),
                location: "New York".to_string(),
                start_date: "2021-03".to_string(),
                end_date: None,
                description: "Leading platform architecture".to_string(),
                tasks: vec!["Architecture design".to_string()],
                achievements: vec!["Scaled to 1M users".to_string()],
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "proj-2".to_string(),
                title: "Microservices Platform".to_string(),
                slug: "microservices-platform".to_string(),
                short_description: "Built enterprise platform".to_string(),
            }],
        };

        update_uc.set_success(updated_cv.clone()).await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(update_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(UpdateCVRequest {
                bio: "Updated comprehensive bio".to_string(),
                role: "Lead Software Architect".to_string(),
                photo_url: "https://example.com/updated.jpg".to_string(),
                core_skills: vec![CoreSkill {
                    title: "System Design".to_string(),
                    description: "Expert in distributed systems".to_string(),
                }],
                educations: vec![EducationRequest {
                    degree: "Master of Computer Science".to_string(),
                    institution: "Stanford".to_string(),
                    graduation_year: 2020,
                }],
                experiences: vec![ExperienceRequest {
                    company: "Big Tech Inc".to_string(),
                    position: "Tech Lead".to_string(),
                    location: "New York".to_string(),
                    start_date: "2021-03".to_string(),
                    end_date: None,
                    description: "Leading platform architecture".to_string(),
                    tasks: vec!["Architecture design".to_string()],
                    achievements: vec!["Scaled to 1M users".to_string()],
                }],
                highlighted_projects: vec![HighlightedProjectRequest {
                    id: "proj-2".to_string(),
                    title: "Microservices Platform".to_string(),
                    slug: "microservices-platform".to_string(),
                    short_description: "Built enterprise platform".to_string(),
                }],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.core_skills.len(), 1);
        assert_eq!(body.educations.len(), 1);
        assert_eq!(body.experiences.len(), 1);
        assert_eq!(body.highlighted_projects.len(), 1);
    }

    #[actix_web::test]
    async fn test_update_cv_handler_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = MockUpdateCVUseCase::new();
        update_uc.set_error(UpdateCVError::CVNotFound).await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(update_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(UpdateCVRequest {
                bio: "Updated bio".to_string(),
                role: "New role".to_string(),
                photo_url: "https://example.com/updated.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 404);

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert_eq!(body_str, "CV not found");
    }

    #[actix_web::test]
    async fn test_update_cv_handler_repository_error() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = MockUpdateCVUseCase::new();
        update_uc
            .set_error(UpdateCVError::RepositoryError(
                "DB update failed".to_string(),
            ))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(update_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(UpdateCVRequest {
                bio: "Updated bio".to_string(),
                role: "Data Engineer".to_string(),
                photo_url: "https://example.com/updated.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 500);

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("DB update failed"));
    }

    #[actix_web::test]
    async fn test_update_cv_handler_missing_authorization() {
        // Arrange
        let cv_id = Uuid::new_v4();

        let update_uc = MockUpdateCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(update_cv_handler),
        )
        .await;

        // Act - No Authorization header
        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .set_json(UpdateCVRequest {
                bio: "Updated bio".to_string(),
                role: "New role".to_string(),
                photo_url: "https://example.com/updated.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_update_cv_handler_invalid_token() {
        // Arrange
        let cv_id = Uuid::new_v4();

        let update_uc = MockUpdateCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(update_cv_handler),
        )
        .await;

        // Act - Invalid token
        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .set_json(UpdateCVRequest {
                bio: "Updated bio".to_string(),
                role: "New role".to_string(),
                photo_url: "https://example.com/updated.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_update_cv_handler_unverified_user() {
        // Arrange
        let user_id = Uuid::new_v4();
        let cv_id = Uuid::new_v4();

        let update_uc = MockUpdateCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_update_cv(update_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        // Generate token for UNVERIFIED user (is_verified = false)
        let token = jwt_service
            .generate_access_token(user_id, false)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(update_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::put()
            .uri(&format!("/api/cvs/{}", cv_id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(UpdateCVRequest {
                bio: "Updated bio".to_string(),
                role: "New role".to_string(),
                photo_url: "https://example.com/updated.jpg".to_string(),
                core_skills: vec![],
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 403); // Forbidden - user not verified
    }
}

use crate::auth::adapter::incoming::web::extractors::auth::VerifiedUser;
use crate::cv::application::ports::outgoing::CreateCVData;
use crate::cv::application::use_cases::create_cv::CreateCVError;
use crate::cv::domain::entities::{CoreSkill, Education, Experience, HighlightedProject};
use crate::AppState;
use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CreateCVRequest {
    pub role: String,
    pub bio: String,
    pub photo_url: String,
    pub core_skills: Vec<CoreSkill>,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
}

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

#[post("/api/cvs")]
pub async fn create_cv_handler(
    user: VerifiedUser,
    req: web::Json<CreateCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    // Map the request fields to domain objects
    let cv_data = CreateCVData {
        role: req.role.clone(),
        bio: req.bio.clone(),
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

    // Call the use case
    match data.create_cv_use_case.execute(user.user_id, cv_data).await {
        Ok(created) => HttpResponse::Created().json(created),

        Err(CreateCVError::RepositoryError(e)) => HttpResponse::InternalServerError().body(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cv::{application::use_cases::create_cv::ICreateCVUseCase, domain::CVInfo},
        tests::support::{
            app_state_builder::TestAppStateBuilder,
            auth_helper::test_helpers::create_test_jwt_service,
        },
    };
    use async_trait::async_trait;
    use std::sync::Arc;

    use super::*;
    use actix_web::{test, web, App};
    use tokio::sync::Mutex;
    use uuid::Uuid;

    // Mock Create CV Use Case
    #[derive(Clone)]
    struct MockCreateCVUseCase {
        should_fail: Arc<Mutex<Option<CreateCVError>>>,
        created_cv: Arc<Mutex<Option<CVInfo>>>,
    }

    impl MockCreateCVUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                created_cv: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: CreateCVError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self, cv: CVInfo) {
            *self.created_cv.lock().await = Some(cv);
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl ICreateCVUseCase for MockCreateCVUseCase {
        async fn execute(
            &self,
            _user_id: Uuid,
            cv_data: CreateCVData,
        ) -> Result<CVInfo, CreateCVError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }

            let cv = self.created_cv.lock().await;
            if let Some(c) = cv.as_ref() {
                return Ok(c.clone());
            }

            // Convert CreateCVData to CVInfo by adding a generated ID
            Ok(CVInfo {
                id: Uuid::new_v4(),
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
    async fn test_create_cv_handler_success() {
        // Arrange
        let user_id = Uuid::new_v4();

        let new_cv = CVInfo {
            id: Uuid::new_v4(),
            user_id,
            bio: "New bio".to_string(),
            role: "New role".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            core_skills: vec![],
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let create_uc = {
            let uc = MockCreateCVUseCase::new();
            uc.set_success(new_cv.clone()).await;
            uc
        };

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(create_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(create_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(CreateCVRequest {
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
        assert_eq!(resp.status(), 201);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.bio, new_cv.bio);
        assert_eq!(body.role, new_cv.role);
        assert_eq!(body.photo_url, new_cv.photo_url);
    }

    #[actix_web::test]
    async fn test_create_cv_handler_with_full_data() {
        // Arrange
        let user_id = Uuid::new_v4();

        let new_cv = CVInfo {
            id: Uuid::new_v4(),
            user_id,
            bio: "Experienced developer".to_string(),
            role: "Senior Software Engineer".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            core_skills: vec![CoreSkill {
                title: "Rust".to_string(),
                description: "Expert in Rust programming".to_string(),
            }],
            educations: vec![Education {
                degree: "Bachelor of Computer Science".to_string(),
                institution: "MIT".to_string(),
                graduation_year: 2018,
            }],
            experiences: vec![Experience {
                company: "Tech Corp".to_string(),
                position: "Senior Developer".to_string(),
                location: "San Francisco".to_string(),
                start_date: "2020-01".to_string(),
                end_date: Some("2023-12".to_string()),
                description: "Led backend development".to_string(),
                tasks: vec![
                    "API design".to_string(),
                    "Database optimization".to_string(),
                ],
                achievements: vec!["Improved performance by 50%".to_string()],
            }],
            highlighted_projects: vec![HighlightedProject {
                id: "proj-1".to_string(),
                title: "E-commerce Platform".to_string(),
                slug: "ecommerce-platform".to_string(),
                short_description: "Built scalable platform".to_string(),
            }],
        };

        let create_uc = {
            let uc = MockCreateCVUseCase::new();
            uc.set_success(new_cv.clone()).await;
            uc
        };

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(create_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(create_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(CreateCVRequest {
                bio: "Experienced developer".to_string(),
                role: "Senior Software Engineer".to_string(),
                photo_url: "https://example.com/photo.jpg".to_string(),
                core_skills: vec![CoreSkill {
                    title: "Rust".to_string(),
                    description: "Expert in Rust programming".to_string(),
                }],
                educations: vec![EducationRequest {
                    degree: "Bachelor of Computer Science".to_string(),
                    institution: "MIT".to_string(),
                    graduation_year: 2018,
                }],
                experiences: vec![ExperienceRequest {
                    company: "Tech Corp".to_string(),
                    position: "Senior Developer".to_string(),
                    location: "San Francisco".to_string(),
                    start_date: "2020-01".to_string(),
                    end_date: Some("2023-12".to_string()),
                    description: "Led backend development".to_string(),
                    tasks: vec![
                        "API design".to_string(),
                        "Database optimization".to_string(),
                    ],
                    achievements: vec!["Improved performance by 50%".to_string()],
                }],
                highlighted_projects: vec![HighlightedProjectRequest {
                    id: "proj-1".to_string(),
                    title: "E-commerce Platform".to_string(),
                    slug: "ecommerce-platform".to_string(),
                    short_description: "Built scalable platform".to_string(),
                }],
            })
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert_eq!(resp.status(), 201);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.core_skills.len(), 1);
        assert_eq!(body.educations.len(), 1);
        assert_eq!(body.experiences.len(), 1);
        assert_eq!(body.highlighted_projects.len(), 1);
    }

    #[actix_web::test]
    async fn test_create_cv_handler_repository_error() {
        // Arrange
        let user_id = Uuid::new_v4();

        let create_uc = MockCreateCVUseCase::new();
        create_uc
            .set_error(CreateCVError::RepositoryError(
                "DB insert failed".to_string(),
            ))
            .await;

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(create_uc)
            .build();

        let jwt_service = create_test_jwt_service();
        let token = jwt_service
            .generate_access_token(user_id, true)
            .expect("Failed to generate token");

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(create_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(CreateCVRequest {
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
        assert_eq!(resp.status(), 500);

        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("DB insert failed"));
    }

    #[actix_web::test]
    async fn test_create_cv_handler_missing_authorization() {
        // Arrange
        let create_uc = MockCreateCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(create_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(create_cv_handler),
        )
        .await;

        // Act - No Authorization header
        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .set_json(CreateCVRequest {
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
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_create_cv_handler_invalid_token() {
        // Arrange
        let create_uc = MockCreateCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(create_uc)
            .build();

        let jwt_service = create_test_jwt_service();

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .app_data(web::Data::new(jwt_service))
                .service(create_cv_handler),
        )
        .await;

        // Act - Invalid token
        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", "Bearer invalid_token_here"))
            .set_json(CreateCVRequest {
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
        assert_eq!(resp.status(), 401); // Unauthorized
    }

    #[actix_web::test]
    async fn test_create_cv_handler_unverified_user() {
        // Arrange
        let user_id = Uuid::new_v4();

        let create_uc = MockCreateCVUseCase::new();

        let app_state = TestAppStateBuilder::default()
            .with_create_cv(create_uc)
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
                .service(create_cv_handler),
        )
        .await;

        // Act
        let req = test::TestRequest::post()
            .uri("/api/cvs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(CreateCVRequest {
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
        assert_eq!(resp.status(), 403); // Forbidden - user not verified
    }
}

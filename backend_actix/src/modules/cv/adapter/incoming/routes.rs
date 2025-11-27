use crate::cv::application::use_cases::create_cv::{CreateCVError, ICreateCVUseCase};
use crate::cv::application::use_cases::fetch_cv::{FetchCVError, IFetchCVUseCase};
use crate::cv::application::use_cases::update_cv::{IUpdateCVUseCase, UpdateCVError};
use crate::cv::domain::entities::{CVInfo, Education, Experience, HighlightedProject};
use crate::AppState;
use actix_web::{get, post, put, web, HttpResponse, Responder};
use uuid::Uuid;

#[get("/api/cv/{user_id}")]
pub async fn get_cv_handler(
    path: web::Path<Uuid>,
    data: web::Data<AppState>, // The state from .app_data(...)
) -> impl Responder {
    let user_id = path.into_inner();
    // 1) Call the existing use case from AppState
    let result = data.fetch_cv_use_case.execute(user_id).await;

    // 2) Map the result to an HTTP response
    match result {
        Ok(cv_info) => HttpResponse::Ok().json(cv_info),
        Err(FetchCVError::CVNotFound) => HttpResponse::NotFound().finish(),
        Err(FetchCVError::RepositoryError(err_msg)) => {
            HttpResponse::InternalServerError().body(err_msg)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct CreateCVRequest {
    pub bio: String,
    pub photo_url: String,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
}

#[derive(serde::Deserialize)]
pub struct EducationRequest {
    pub degree: String,
    pub institution: String,
    pub graduation_year: i32,
}

#[derive(serde::Deserialize)]
pub struct ExperienceRequest {
    pub company: String,
    pub position: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: String,
}

#[derive(serde::Deserialize)]
pub struct HighlightedProjectRequest {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: String,
}

#[post("/api/cv/{user_id}")]
pub async fn create_cv_handler(
    path: web::Path<Uuid>,
    req: web::Json<CreateCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let user_id = path.into_inner();

    // Map the request fields to domain objects
    let cv_data = CVInfo {
        bio: req.bio.clone(),
        photo_url: req.photo_url.clone(),
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
                start_date: exp.start_date.clone(),
                end_date: exp.end_date.clone(),
                description: exp.description.clone(),
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
    match data.create_cv_use_case.execute(user_id, cv_data).await {
        Ok(created) => HttpResponse::Created().json(created),
        Err(CreateCVError::AlreadyExists) => HttpResponse::Conflict().body("CV already exists"),
        Err(CreateCVError::RepositoryError(e)) => HttpResponse::InternalServerError().body(e),
    }
}

#[derive(serde::Deserialize)]
pub struct UpdateCVRequest {
    pub bio: String,
    pub photo_url: String,
    pub educations: Vec<EducationRequest>,
    pub experiences: Vec<ExperienceRequest>,
    pub highlighted_projects: Vec<HighlightedProjectRequest>,
}

#[put("/api/cv/{user_id}")]
pub async fn update_cv_handler(
    path: web::Path<Uuid>,
    req: web::Json<UpdateCVRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let user_id = path.into_inner();

    let cv_data = CVInfo {
        bio: req.bio.clone(),
        photo_url: req.photo_url.clone(),
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
                start_date: exp.start_date.clone(),
                end_date: exp.end_date.clone(),
                description: exp.description.clone(),
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

    match data.update_cv_use_case.execute(user_id, cv_data).await {
        Ok(updated) => HttpResponse::Ok().json(updated),
        Err(UpdateCVError::CVNotFound) => HttpResponse::NotFound().body("CV not found"),
        Err(UpdateCVError::RepositoryError(e)) => HttpResponse::InternalServerError().body(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cv::domain::entities::CVInfo;
    use crate::modules::auth::application::use_cases::create_user::{
        CreateUserError, ICreateUserUseCase,
    };
    use crate::modules::auth::application::use_cases::verify_user_email::{
        IVerifyUserEmailUseCase, VerifyUserEmailError,
    };
    use crate::modules::cv::application::use_cases::{
        create_cv::{CreateCVError, ICreateCVUseCase},
        fetch_cv::{FetchCVError, IFetchCVUseCase},
        update_cv::{IUpdateCVUseCase, UpdateCVError},
    };
    use actix_web::{test, web, App};
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    // ==================== MOCK IMPLEMENTATIONS ====================

    // Mock Fetch CV Use Case
    #[derive(Clone)]
    struct MockFetchCVUseCase {
        should_fail: Arc<Mutex<Option<FetchCVError>>>,
        cv: Arc<Mutex<Option<CVInfo>>>,
    }

    impl MockFetchCVUseCase {
        fn new() -> Self {
            Self {
                should_fail: Arc::new(Mutex::new(None)),
                cv: Arc::new(Mutex::new(None)),
            }
        }

        async fn set_error(&self, error: FetchCVError) {
            *self.should_fail.lock().await = Some(error);
        }

        async fn set_success(&self, cv: CVInfo) {
            *self.cv.lock().await = Some(cv);
            *self.should_fail.lock().await = None;
        }
    }

    #[async_trait]
    impl IFetchCVUseCase for MockFetchCVUseCase {
        async fn execute(&self, _user_id: Uuid) -> Result<CVInfo, FetchCVError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }

            let cv = self.cv.lock().await;
            if let Some(c) = cv.as_ref() {
                return Ok(c.clone());
            }

            // Default success case
            Ok(CVInfo {
                bio: "Default bio".to_string(),
                photo_url: "https://example.com/photo.jpg".to_string(),
                educations: vec![],
                experiences: vec![],
                highlighted_projects: vec![],
            })
        }
    }

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
        async fn execute(&self, _user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, CreateCVError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }

            let cv = self.created_cv.lock().await;
            if let Some(c) = cv.as_ref() {
                return Ok(c.clone());
            }

            // Return the provided CV data
            Ok(cv_data)
        }
    }

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
        async fn execute(&self, _user_id: Uuid, cv_data: CVInfo) -> Result<CVInfo, UpdateCVError> {
            let error = self.should_fail.lock().await;
            if let Some(err) = error.as_ref() {
                return Err(err.clone());
            }

            let cv = self.updated_cv.lock().await;
            if let Some(c) = cv.as_ref() {
                return Ok(c.clone());
            }

            // Return the updated CV data
            Ok(cv_data)
        }
    }

    // Stub implementations for auth use cases
    #[derive(Default, Clone)]
    struct StubCreateUserUseCase;

    #[async_trait]
    impl ICreateUserUseCase for StubCreateUserUseCase {
        async fn execute(
            &self,
            _username: String,
            _email: String,
            _password: String,
        ) -> Result<User, CreateUserError> {
            unimplemented!("Not used in CV tests")
        }
    }

    #[derive(Default, Clone)]
    struct StubVerifyUserEmailUseCase;

    #[async_trait]
    impl IVerifyUserEmailUseCase for StubVerifyUserEmailUseCase {
        async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
            unimplemented!("Not used in CV tests")
        }
    }

    // Helper to create test AppState
    fn create_test_app_state(
        fetch_cv_uc: MockFetchCVUseCase,
        create_cv_uc: MockCreateCVUseCase,
        update_cv_uc: MockUpdateCVUseCase,
    ) -> web::Data<AppState> {
        web::Data::new(AppState {
            fetch_cv_use_case: Arc::new(fetch_cv_uc),
            create_cv_use_case: Arc::new(create_cv_uc),
            update_cv_use_case: Arc::new(update_cv_uc),
            create_user_use_case: Arc::new(StubCreateUserUseCase::default()),
            verify_user_email_use_case: Arc::new(StubVerifyUserEmailUseCase::default()),
        })
    }

    // ==================== FETCH CV TESTS ====================

    #[actix_web::test]
    async fn test_fetch_cv_handler_success() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        let expected_cv = CVInfo {
            bio: "Software Engineer".to_string(),
            photo_url: "https://example.com/photo.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        fetch_uc.set_success(expected_cv.clone()).await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let app = test::init_service(
            App::new().app_data(app_state).service(fetch_cv_handler), // Your CV route handler
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cv/{}", user_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.bio, expected_cv.bio);
    }

    #[actix_web::test]
    async fn test_fetch_cv_handler_not_found() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        fetch_uc.set_error(FetchCVError::NotFound).await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let app =
            test::init_service(App::new().app_data(app_state).service(fetch_cv_handler)).await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cv/{}", user_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_fetch_cv_handler_database_error() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        fetch_uc
            .set_error(FetchCVError::RepositoryError("DB error".to_string()))
            .await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let app =
            test::init_service(App::new().app_data(app_state).service(fetch_cv_handler)).await;

        let req = test::TestRequest::get()
            .uri(&format!("/api/cv/{}", user_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);
    }

    // ==================== CREATE CV TESTS ====================

    #[actix_web::test]
    async fn test_create_cv_handler_success() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        let new_cv = CVInfo {
            bio: "New bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        create_uc.set_success(new_cv.clone()).await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let app =
            test::init_service(App::new().app_data(app_state).service(create_cv_handler)).await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/cv/{}", user_id))
            .set_json(&new_cv)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.bio, new_cv.bio);
    }

    #[actix_web::test]
    async fn test_create_cv_handler_already_exists() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        create_uc.set_error(CreateCVError::AlreadyExists).await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let new_cv = CVInfo {
            bio: "New bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let app =
            test::init_service(App::new().app_data(app_state).service(create_cv_handler)).await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/cv/{}", user_id))
            .set_json(&new_cv)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 409); // Conflict
    }

    #[actix_web::test]
    async fn test_create_cv_handler_repository_error() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        create_uc
            .set_error(CreateCVError::RepositoryError("DB error".to_string()))
            .await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let new_cv = CVInfo {
            bio: "New bio".to_string(),
            photo_url: "https://example.com/new.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let app =
            test::init_service(App::new().app_data(app_state).service(create_cv_handler)).await;

        let req = test::TestRequest::post()
            .uri(&format!("/api/cv/{}", user_id))
            .set_json(&new_cv)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);
    }

    // ==================== UPDATE CV TESTS ====================

    #[actix_web::test]
    async fn test_update_cv_handler_success() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        let updated_cv = CVInfo {
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        update_uc.set_success(updated_cv.clone()).await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let app =
            test::init_service(App::new().app_data(app_state).service(update_cv_handler)).await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cv/{}", user_id))
            .set_json(&updated_cv)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: CVInfo = test::read_body_json(resp).await;
        assert_eq!(body.bio, updated_cv.bio);
    }

    #[actix_web::test]
    async fn test_update_cv_handler_not_found() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        update_uc.set_error(UpdateCVError::NotFound).await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let updated_cv = CVInfo {
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let app =
            test::init_service(App::new().app_data(app_state).service(update_cv_handler)).await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cv/{}", user_id))
            .set_json(&updated_cv)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }

    #[actix_web::test]
    async fn test_update_cv_handler_repository_error() {
        let fetch_uc = MockFetchCVUseCase::new();
        let create_uc = MockCreateCVUseCase::new();
        let update_uc = MockUpdateCVUseCase::new();

        update_uc
            .set_error(UpdateCVError::RepositoryError("DB error".to_string()))
            .await;
        let app_state = create_test_app_state(fetch_uc, create_uc, update_uc);

        let user_id = Uuid::new_v4();
        let updated_cv = CVInfo {
            bio: "Updated bio".to_string(),
            photo_url: "https://example.com/updated.jpg".to_string(),
            educations: vec![],
            experiences: vec![],
            highlighted_projects: vec![],
        };

        let app =
            test::init_service(App::new().app_data(app_state).service(update_cv_handler)).await;

        let req = test::TestRequest::put()
            .uri(&format!("/api/cv/{}", user_id))
            .set_json(&updated_cv)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 500);
    }
}

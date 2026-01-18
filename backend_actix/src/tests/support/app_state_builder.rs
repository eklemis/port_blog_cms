use crate::auth::application::use_cases::refresh_token::IRefreshTokenUseCase;
use crate::auth::application::use_cases::{
    create_user::ICreateUserUseCase, login_user::ILoginUserUseCase, logout_user::ILogoutUseCase,
    verify_user_email::IVerifyUserEmailUseCase,
};
use crate::cv::application::use_cases::create_cv::ICreateCVUseCase;
use crate::cv::application::use_cases::fetch_cv_by_id::IFetchCVByIdUseCase;
use crate::cv::application::use_cases::fetch_user_cvs::IFetchCVUseCase;
use crate::cv::application::use_cases::patch_cv::IPatchCVUseCase;
use crate::cv::application::use_cases::update_cv::IUpdateCVUseCase;
use crate::tests::support::stubs::*;
use crate::AppState;
use actix_web::web;
use std::sync::Arc;

pub struct TestAppStateBuilder {
    fetch_cv: Option<Arc<dyn IFetchCVUseCase + Send + Sync>>,
    fetch_cv_by_id: Option<Arc<dyn IFetchCVByIdUseCase + Send + Sync>>,
    create_cv: Option<Arc<dyn ICreateCVUseCase + Send + Sync>>,
    update_cv: Option<Arc<dyn IUpdateCVUseCase + Send + Sync>>,
    patch_cv: Option<Arc<dyn IPatchCVUseCase + Send + Sync>>,
    create_user: Option<Arc<dyn ICreateUserUseCase + Send + Sync>>,
    verify_user_email: Option<Arc<dyn IVerifyUserEmailUseCase + Send + Sync>>,
    login_user: Option<Arc<dyn ILoginUserUseCase + Send + Sync>>,
    refresh_token: Option<Arc<dyn IRefreshTokenUseCase + Send + Sync>>,
    logout_user: Option<Arc<dyn ILogoutUseCase + Send + Sync>>,
}

impl Default for TestAppStateBuilder {
    fn default() -> Self {
        Self {
            fetch_cv: Some(Arc::new(StubFetchCVUseCase)),
            fetch_cv_by_id: Some(Arc::new(StubFetchCVByIdUseCase)),
            create_cv: Some(Arc::new(StubCreateCVUseCase)),
            update_cv: Some(Arc::new(StubUpdateCVUseCase)),
            patch_cv: Some(Arc::new(StubPatchCVUseCase)),
            create_user: Some(Arc::new(StubCreateUserUseCase)),
            verify_user_email: Some(Arc::new(StubVerifyUserEmailUseCase)),
            login_user: Some(Arc::new(StubLoginUserUseCase)),
            refresh_token: Some(Arc::new(StubRefreshTokenUseCase)),
            logout_user: Some(Arc::new(StubLogoutUserUseCase)),
        }
    }
}

impl TestAppStateBuilder {
    pub fn with_fetch_cv(mut self, uc: impl IFetchCVUseCase + Send + Sync + 'static) -> Self {
        self.fetch_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_fetch_cv_by_id(
        mut self,
        uc: impl IFetchCVByIdUseCase + Send + Sync + 'static,
    ) -> Self {
        self.fetch_cv_by_id = Some(Arc::new(uc));
        self
    }

    pub fn with_create_cv(mut self, uc: impl ICreateCVUseCase + Send + Sync + 'static) -> Self {
        self.create_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_update_cv(mut self, uc: impl IUpdateCVUseCase + Send + Sync + 'static) -> Self {
        self.update_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_patch_cv(mut self, uc: impl IPatchCVUseCase + Send + Sync + 'static) -> Self {
        self.patch_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_create_user(mut self, uc: impl ICreateUserUseCase + Send + Sync + 'static) -> Self {
        self.create_user = Some(Arc::new(uc));
        self
    }

    pub fn with_login_user(mut self, uc: impl ILoginUserUseCase + Send + Sync + 'static) -> Self {
        self.login_user = Some(Arc::new(uc));
        self
    }

    pub fn with_verify_user_email(
        mut self,
        uc: impl IVerifyUserEmailUseCase + Send + Sync + 'static,
    ) -> Self {
        self.verify_user_email = Some(Arc::new(uc));
        self
    }

    pub fn with_refresh_token(
        mut self,
        uc: impl IRefreshTokenUseCase + Send + Sync + 'static,
    ) -> Self {
        self.refresh_token = Some(Arc::new(uc));
        self
    }

    pub fn with_logout_user(mut self, uc: impl ILogoutUseCase + Send + Sync + 'static) -> Self {
        self.logout_user = Some(Arc::new(uc));
        self
    }

    pub fn build(self) -> web::Data<AppState> {
        web::Data::new(AppState {
            // Safe unwrap: defaults are always set in Default
            fetch_cv_use_case: self.fetch_cv.unwrap(),
            fetch_cv_by_id_use_case: self.fetch_cv_by_id.unwrap(),
            create_cv_use_case: self.create_cv.unwrap(),
            update_cv_use_case: self.update_cv.unwrap(),
            patch_cv_use_case: self.patch_cv.unwrap(),
            create_user_use_case: self.create_user.unwrap(),
            verify_user_email_use_case: self.verify_user_email.unwrap(),
            login_user_use_case: self.login_user.unwrap(),
            refresh_token_use_case: self.refresh_token.unwrap(),
            logout_user_use_case: self.logout_user.unwrap(),
        })
    }
}

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::auth::application::use_cases::create_user::{CreateUserInput, CreateUserOutput};
use crate::auth::application::use_cases::fetch_profile::{
    FetchUserError, FetchUserOutput, FetchUserProfileUseCase,
};
use crate::auth::application::use_cases::logout_user::{
    LogoutError, LogoutRequest, LogoutResponse,
};
use crate::auth::application::use_cases::refresh_token::{
    IRefreshTokenUseCase, RefreshTokenError, RefreshTokenRequest, RefreshTokenResponse,
};
use crate::auth::application::use_cases::soft_delete_user::{
    ISoftDeleteUserUseCase, SoftDeleteUserError, SoftDeleteUserRequest,
};
use crate::auth::application::use_cases::update_profile::{
    UpdateUserError, UpdateUserInput, UpdateUserOutput, UpdateUserProfileUseCase,
};
use crate::cv::application::use_cases::hard_delete_cv::{HardDeleteCVError, HardDeleteCvUseCase};
use crate::cv::application::use_cases::restore_cv::{RestoreCVError, RestoreDeletedCvUseCase};
use crate::cv::application::use_cases::soft_delete_cv::{SoftDeleteCVError, SoftDeleteCvUseCase};
use crate::cv::domain::entities::CVInfo;
use crate::email::application::ports::outgoing::user_email_notifier::{
    UserEmailNotificationError, UserEmailNotifier,
};
use crate::{
    auth::application::use_cases::login_user::{LoginError, LoginRequest, LoginUserResponse},
    cv::application::use_cases::{
        create_cv::{CreateCVError, ICreateCVUseCase},
        fetch_cv_by_id::{FetchCVByIdError, IFetchCVByIdUseCase},
        fetch_user_cvs::{FetchCVError, IFetchCVUseCase},
        patch_cv::{IPatchCVUseCase, PatchCVError},
        update_cv::{IUpdateCVUseCase, UpdateCVError},
    },
};

use crate::auth::application::use_cases::{
    create_user::{CreateUserError, ICreateUserUseCase},
    login_user::ILoginUserUseCase,
    logout_user::ILogoutUseCase,
    verify_user_email::{IVerifyUserEmailUseCase, VerifyUserEmailError},
};

#[derive(Default, Clone)]
pub struct StubFetchCVUseCase;

#[async_trait]
impl IFetchCVUseCase for StubFetchCVUseCase {
    async fn execute(&self, _user_id: Uuid) -> Result<Vec<CVInfo>, FetchCVError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Default, Clone)]
pub struct StubFetchCVByIdUseCase;

#[async_trait]
impl IFetchCVByIdUseCase for StubFetchCVByIdUseCase {
    async fn execute(&self, _user_id: Uuid, _cv_id: Uuid) -> Result<CVInfo, FetchCVByIdError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Default, Clone)]
pub struct StubCreateCVUseCase;

#[async_trait]
impl ICreateCVUseCase for StubCreateCVUseCase {
    async fn execute(
        &self,
        _user_id: Uuid,
        _data: crate::cv::application::ports::outgoing::CreateCVData,
    ) -> Result<CVInfo, CreateCVError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Default, Clone)]
pub struct StubUpdateCVUseCase;

#[async_trait]
impl IUpdateCVUseCase for StubUpdateCVUseCase {
    async fn execute(
        &self,
        _user_id: Uuid,
        _cv_id: Uuid,
        _data: crate::cv::application::ports::outgoing::UpdateCVData,
    ) -> Result<CVInfo, UpdateCVError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Default, Clone)]
pub struct StubPatchCVUseCase;

#[async_trait]
impl IPatchCVUseCase for StubPatchCVUseCase {
    async fn execute(
        &self,
        _user_id: Uuid,
        _cv_id: Uuid,
        _data: crate::cv::application::ports::outgoing::PatchCVData,
    ) -> Result<CVInfo, PatchCVError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Default, Clone)]
pub struct StubCreateUserUseCase;

#[async_trait]
impl ICreateUserUseCase for StubCreateUserUseCase {
    async fn execute(&self, _input: CreateUserInput) -> Result<CreateUserOutput, CreateUserError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Default, Clone)]
pub struct StubVerifyUserEmailUseCase;

#[async_trait]
impl IVerifyUserEmailUseCase for StubVerifyUserEmailUseCase {
    async fn execute(&self, _token: &str) -> Result<(), VerifyUserEmailError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Default, Clone)]
pub struct StubLoginUserUseCase;

#[async_trait]
impl ILoginUserUseCase for StubLoginUserUseCase {
    async fn execute(&self, _request: LoginRequest) -> Result<LoginUserResponse, LoginError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubRefreshTokenUseCase;

#[async_trait]
impl IRefreshTokenUseCase for StubRefreshTokenUseCase {
    async fn execute(
        &self,
        _request: RefreshTokenRequest,
    ) -> Result<RefreshTokenResponse, RefreshTokenError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubLogoutUserUseCase;

#[async_trait]
impl ILogoutUseCase for StubLogoutUserUseCase {
    async fn execute(&self, _request: LogoutRequest) -> Result<LogoutResponse, LogoutError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubSoftDeleteUserUseCase;

#[async_trait]
impl ISoftDeleteUserUseCase for StubSoftDeleteUserUseCase {
    async fn execute(&self, _request: SoftDeleteUserRequest) -> Result<(), SoftDeleteUserError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubUserEmailNotifier;

#[async_trait]
impl UserEmailNotifier for StubUserEmailNotifier {
    async fn send_verification_email(
        &self,
        _user: CreateUserOutput,
    ) -> Result<(), UserEmailNotificationError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubFetchUserProfileUseCase;

#[async_trait]
impl FetchUserProfileUseCase for StubFetchUserProfileUseCase {
    async fn execute(&self, user_id: UserId) -> Result<FetchUserOutput, FetchUserError> {
        Ok(FetchUserOutput {
            user_id,
            email: "stub@example.com".to_string(),
            username: "stubuser".to_string(),
            full_name: "Stub User".to_string(),
        })
    }
}

#[derive(Default, Clone)]
pub struct StubUpdateUserProfileUseCase;

#[async_trait]
impl UpdateUserProfileUseCase for StubUpdateUserProfileUseCase {
    async fn execute(&self, _data: UpdateUserInput) -> Result<UpdateUserOutput, UpdateUserError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubHardDeleteCv;

#[async_trait]
impl HardDeleteCvUseCase for StubHardDeleteCv {
    async fn execute(&self, _user_id: UserId, _cv_id: Uuid) -> Result<(), HardDeleteCVError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubSoftDeleteCv;

#[async_trait]
impl SoftDeleteCvUseCase for StubSoftDeleteCv {
    async fn execute(&self, _user_id: UserId, _cv_id: Uuid) -> Result<(), SoftDeleteCVError> {
        unimplemented!()
    }
}

#[derive(Default, Clone)]
pub struct StubResotoreDeletedCv;

#[async_trait]
impl RestoreDeletedCvUseCase for StubResotoreDeletedCv {
    async fn execute(&self, _user_id: UserId, _cv_id: Uuid) -> Result<(), RestoreCVError> {
        unimplemented!()
    }
}

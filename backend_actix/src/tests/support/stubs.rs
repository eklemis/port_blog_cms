use async_trait::async_trait;
use uuid::Uuid;

use crate::cv::domain::entities::CVInfo;
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
    verify_user_email::{IVerifyUserEmailUseCase, VerifyUserEmailError},
};

use crate::modules::auth::application::domain::entities::User;

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
    async fn execute(
        &self,
        _username: String,
        _email: String,
        _password: String,
    ) -> Result<User, CreateUserError> {
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

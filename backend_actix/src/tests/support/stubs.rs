use crate::auth::application::use_cases::request_password_reset::{
    IRequestPasswordResetUseCase, RequestPasswordResetError,
};
use crate::auth::application::use_cases::reset_password::{
    IResetPasswordUseCase, ResetPasswordError,
};
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::application::domain::entities::UserId;
use crate::auth::application::ports::outgoing::token_provider::{
    TokenClaims, TokenError, TokenProvider,
};
use crate::auth::application::ports::outgoing::user_query::{UserQueryError, UserQueryResult};
use crate::auth::application::ports::outgoing::UserQuery;
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
use crate::blog::application::ports::incoming::use_cases::{
    ArchiveBlogPostError, ArchiveBlogPostUseCase, AttachBlogPostTopicUseCase, BlogPostTopicError,
    ClearBlogPostTopicsUseCase, CreateBlogPostCommand, CreateBlogPostError, CreateBlogPostUseCase,
    DetachBlogPostTopicUseCase, GetBlogPostError, GetBlogPostTopicsUseCase, GetBlogPostsError,
    GetBlogPostsUseCase, GetPublicBlogPostUseCase, GetPublicBlogPostsUseCase,
    GetSingleBlogPostUseCase, HardDeleteBlogPostUseCase, PatchBlogPostError, PatchBlogPostUseCase,
    RestoreBlogPostUseCase,
};
use crate::blog::application::ports::outgoing::{
    BlogPageRequest, BlogPageResult, BlogPostCard, BlogPostListFilter, BlogPostSort, BlogPostView,
    PatchBlogPostData,
};
use crate::blog::domain::entities::{BlogPost, BlogPostTopic};
use crate::cv::application::ports::outgoing::{CVListFilter, CVPageRequest, CVPageResult, CVSort};
use crate::cv::application::use_cases::get_public_single_cv::{
    GetPublicSingleCvError, GetPublicSingleCvUseCase,
};
use crate::cv::application::use_cases::hard_delete_cv::{HardDeleteCVError, HardDeleteCvUseCase};
use crate::cv::application::use_cases::restore_cv::{RestoreCVError, RestoreDeletedCvUseCase};
use crate::cv::application::use_cases::soft_delete_cv::{SoftDeleteCVError, SoftDeleteCvUseCase};
use crate::cv::domain::entities::CVInfo;
use crate::email::application::ports::outgoing::user_email_notifier::{
    UserEmailNotificationError, UserEmailNotifier,
};
use crate::email::application::ports::outgoing::Recipient;
use crate::multimedia::application::ports::incoming::use_cases::{
    CreateAttachmentCommand, CreateMediaCommand, CreateMediaResult, CreateUploadMediaUrlUseCase,
    CreateUrlError, DeleteMediaError, DeleteMediaUseCase, GetMediaError, GetMediaUseCase,
    GetReadUrlError, GetUrlCommand, GetUrlResult, GetVariantReadUrlUseCase, ListMediaCommand,
    ListMediaError, ListMediaUseCase, MediaDetail, MediaItem,
};

use crate::project::application::ports::incoming::use_cases::{
    AddProjectTopicError, AddProjectTopicUseCase, ClearProjectTopicsError,
    ClearProjectTopicsUseCase, GetProjectTopicsError, GetProjectTopicsUseCase, GetProjectsUseCase,
    GetPublicSingleProjectError, GetPublicSingleProjectUseCase, GetSingleProjectError,
    GetSingleProjectUseCase, HardDeleteProjectError, HardDeleteProjectUseCase, PatchProjectError,
    PatchProjectUseCase, RemoveProjectTopicError, RemoveProjectTopicUseCase,
    SoftDeleteProjectError, SoftDeleteProjectUseCase,
};
use crate::project::application::ports::outgoing::project_query::{ProjectTopicItem, ProjectView};
use crate::project::application::ports::outgoing::project_repository::PatchProjectData;
use crate::tests::support::project_test_fixtures::empty_page_result;
use crate::topic::application::ports::outgoing::TopicResult;
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

use crate::modules::topic::application::ports::incoming::use_cases::{
    CreateTopicCommand, CreateTopicUseCase, GetTopicsUseCase, SoftDeleteTopicUseCase,
};
use crate::modules::topic::application::ports::incoming::use_cases::{
    CreateTopicError, GetTopicsError, SoftDeleteTopicError,
};
use crate::modules::topic::application::ports::outgoing::TopicQueryResult;

#[derive(Default, Clone)]
pub struct StubFetchCVUseCase;

#[async_trait]
impl IFetchCVUseCase for StubFetchCVUseCase {
    async fn execute(
        &self,
        _user_id: Uuid,
        _filter: CVListFilter,
        _sort: CVSort,
        _page: CVPageRequest,
    ) -> Result<CVPageResult<CVInfo>, FetchCVError> {
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
        _recipient: &Recipient,
        _verification_token: &str,
    ) -> Result<(), UserEmailNotificationError> {
        unimplemented!()
    }
}

/// Mints a fixed verification token.
///
/// The registration orchestrator mints the token itself now, so anything that
/// builds one needs a token provider. See
/// `docs/adr/0005-break-the-auth-email-cycle.md`.
#[derive(Default, Clone)]
pub struct StubTokenProvider;

impl TokenProvider for StubTokenProvider {
    fn generate_access_token(&self, _u: uuid::Uuid, _v: bool) -> Result<String, TokenError> {
        Ok("access".to_string())
    }
    fn generate_refresh_token(&self, _u: uuid::Uuid, _v: bool) -> Result<String, TokenError> {
        Ok("refresh".to_string())
    }
    fn verify_token(&self, _t: &str) -> Result<TokenClaims, TokenError> {
        unimplemented!()
    }
    fn refresh_access_token(&self, _t: &str) -> Result<String, TokenError> {
        unimplemented!()
    }
    fn generate_verification_token(&self, _u: uuid::Uuid) -> Result<String, TokenError> {
        Ok("verify".to_string())
    }
    fn verify_verification_token(&self, _t: &str) -> Result<uuid::Uuid, TokenError> {
        unimplemented!()
    }
    fn generate_password_reset_token(&self, _u: uuid::Uuid) -> Result<String, TokenError> {
        Ok("reset".to_string())
    }
    fn verify_password_reset_token(&self, _t: &str) -> Result<uuid::Uuid, TokenError> {
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
    async fn execute(&self, data: UpdateUserInput) -> Result<UpdateUserOutput, UpdateUserError> {
        Ok(UpdateUserOutput {
            user_id: data.user_id,
            email: "stub@example.com".to_string(),
            username: "stubuser".to_string(),
            full_name: data.full_name,
        })
    }
}

#[derive(Default, Clone)]
pub struct StubHardDeleteCvUseCase;

#[async_trait]
impl HardDeleteCvUseCase for StubHardDeleteCvUseCase {
    async fn execute(&self, _user_id: UserId, _cv_id: Uuid) -> Result<(), HardDeleteCVError> {
        Ok(())
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
pub struct StubRestoreDeletedCv;

#[async_trait]
impl RestoreDeletedCvUseCase for StubRestoreDeletedCv {
    async fn execute(&self, _user_id: UserId, _cv_id: Uuid) -> Result<CVInfo, RestoreCVError> {
        unimplemented!("StubRestoreDeletedCv not configured for this test")
    }
}

#[derive(Default, Clone)]
pub struct StubCreateTopicUseCase;

#[async_trait]
impl CreateTopicUseCase for StubCreateTopicUseCase {
    async fn execute(&self, command: CreateTopicCommand) -> Result<TopicResult, CreateTopicError> {
        Ok(TopicResult {
            id: uuid::Uuid::new_v4(),
            owner: *command.owner(),
            title: command.title().to_string(),
            description: command.description().cloned().unwrap_or_default(),
        })
    }
}

#[derive(Clone)]
pub struct StubGetTopicsUseCase {
    result: Result<Vec<TopicQueryResult>, GetTopicsError>,
}

impl StubGetTopicsUseCase {
    pub fn success(data: Vec<TopicQueryResult>) -> Self {
        Self { result: Ok(data) }
    }

    pub fn failure(msg: &str) -> Self {
        Self {
            result: Err(GetTopicsError::QueryFailed(msg.into())),
        }
    }
}

#[async_trait]
impl GetTopicsUseCase for StubGetTopicsUseCase {
    async fn execute(&self, _owner: UserId) -> Result<Vec<TopicQueryResult>, GetTopicsError> {
        self.result.clone()
    }
}

#[derive(Default, Clone)]
pub struct StubSoftDeleteTopicUseCase;

#[async_trait]
impl SoftDeleteTopicUseCase for StubSoftDeleteTopicUseCase {
    async fn execute(&self, _owner: UserId, _topic_id: Uuid) -> Result<(), SoftDeleteTopicError> {
        Ok(())
    }
}

use crate::modules::project::application::ports::incoming::use_cases::{
    CreateProjectError, CreateProjectUseCase,
};
use crate::modules::project::application::ports::outgoing::project_repository::{
    CreateProjectData, ProjectResult,
};

#[derive(Clone)]
pub struct StubCreateProjectUseCase {
    result: Result<ProjectResult, CreateProjectError>,
}

impl StubCreateProjectUseCase {
    pub fn repo_error(msg: &str) -> Self {
        Self {
            result: Err(CreateProjectError::RepositoryError(msg.to_string())),
        }
    }
}

#[async_trait]
impl CreateProjectUseCase for StubCreateProjectUseCase {
    async fn execute(&self, _data: CreateProjectData) -> Result<ProjectResult, CreateProjectError> {
        self.result.clone()
    }
}

#[derive(Default, Clone)]
pub struct DefaultStubGetProjectsUseCase;

#[async_trait]
impl GetProjectsUseCase for DefaultStubGetProjectsUseCase {
    async fn execute(
        &self,
        _owner: crate::auth::application::domain::entities::UserId,
        _filter: crate::modules::project::application::ports::outgoing::project_query::ProjectListFilter,
        _sort: crate::modules::project::application::ports::outgoing::project_query::ProjectSort,
        _page: crate::modules::project::application::ports::outgoing::project_query::PageRequest,
    ) -> Result<
        crate::modules::project::application::ports::outgoing::project_query::PageResult<
            crate::modules::project::application::ports::outgoing::project_query::ProjectCardView,
        >,
        crate::modules::project::application::ports::incoming::use_cases::GetProjectsError,
    > {
        Ok(empty_page_result())
    }
}

#[derive(Clone)]
pub struct StubGetSingleProjectUseCase {
    result: Result<ProjectView, GetSingleProjectError>,
}

impl StubGetSingleProjectUseCase {
    pub fn not_found() -> Self {
        Self {
            result: Err(GetSingleProjectError::NotFound),
        }
    }
}

#[async_trait]
impl GetSingleProjectUseCase for StubGetSingleProjectUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
    ) -> Result<ProjectView, GetSingleProjectError> {
        self.result.clone()
    }
}

#[derive(Default, Clone)]
pub struct DefaultStubPatchProjectUseCase;

#[async_trait]
impl PatchProjectUseCase for DefaultStubPatchProjectUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
        _data: PatchProjectData,
    ) -> Result<ProjectResult, PatchProjectError> {
        unimplemented!("Not used in this test")
    }
}

#[derive(Clone)]
pub struct DummyUserQuery;

#[async_trait]
impl UserQuery for DummyUserQuery {
    async fn find_by_id(&self, _user_id: Uuid) -> Result<Option<UserQueryResult>, UserQueryError> {
        Ok(None)
    }

    async fn find_by_email(&self, _email: &str) -> Result<Option<UserQueryResult>, UserQueryError> {
        Ok(None)
    }

    async fn find_by_username(
        &self,
        _username: &str,
    ) -> Result<Option<UserQueryResult>, UserQueryError> {
        Ok(None)
    }
}

#[derive(Clone)]
pub struct StubGetPublicSingleProjectUseCase {
    result: Result<ProjectView, GetPublicSingleProjectError>,
}

impl StubGetPublicSingleProjectUseCase {
    pub fn not_found() -> Self {
        Self {
            result: Err(GetPublicSingleProjectError::NotFound),
        }
    }
}

#[async_trait]
impl GetPublicSingleProjectUseCase for StubGetPublicSingleProjectUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _slug: &str,
    ) -> Result<ProjectView, GetPublicSingleProjectError> {
        self.result.clone()
    }
}

#[derive(Clone)]
pub struct StubGetPublicSingleCvUseCase {
    pub result: Result<CVInfo, GetPublicSingleCvError>,
}

#[async_trait]
impl GetPublicSingleCvUseCase for StubGetPublicSingleCvUseCase {
    async fn execute(
        &self,
        _owner_id: Uuid,
        _cv_id: Uuid,
    ) -> Result<CVInfo, GetPublicSingleCvError> {
        self.result.clone()
    }
}

impl StubGetPublicSingleCvUseCase {
    pub fn not_found() -> Arc<Self> {
        Arc::new(Self {
            result: Err(GetPublicSingleCvError::NotFound),
        })
    }
}

#[derive(Clone, Default)]
pub struct StubAddProjectTopicUseCase;

#[async_trait]
impl AddProjectTopicUseCase for StubAddProjectTopicUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
        _topic_id: Uuid,
    ) -> Result<(), AddProjectTopicError> {
        unimplemented!("StubAddProjectTopicUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubRemoveProjectTopicUseCase;

#[async_trait]
impl RemoveProjectTopicUseCase for StubRemoveProjectTopicUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
        _topic_id: Uuid,
    ) -> Result<(), RemoveProjectTopicError> {
        unimplemented!("StubRemoveProjectTopicUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubClearProjectTopicsUseCase;

#[async_trait]
impl ClearProjectTopicsUseCase for StubClearProjectTopicsUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
    ) -> Result<(), ClearProjectTopicsError> {
        unimplemented!("StubClearProjectTopicsUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubGetProjectTopicsUseCase;

#[async_trait]
impl GetProjectTopicsUseCase for StubGetProjectTopicsUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
    ) -> Result<Vec<ProjectTopicItem>, GetProjectTopicsError> {
        unimplemented!("StubGetProjectTopicsUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubHardDeleteProjectUseCase;

#[async_trait]
impl HardDeleteProjectUseCase for StubHardDeleteProjectUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
    ) -> Result<(), HardDeleteProjectError> {
        unimplemented!("StubHardDeleteProjectUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubSoftDeleteProjectUseCase;

#[async_trait]
impl SoftDeleteProjectUseCase for StubSoftDeleteProjectUseCase {
    async fn execute(
        &self,
        _owner: UserId,
        _project_id: Uuid,
    ) -> Result<(), SoftDeleteProjectError> {
        unimplemented!("StubSoftDeleteProjectUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubCreateUploadMediaUrlUseCase;

#[async_trait]
impl CreateUploadMediaUrlUseCase for StubCreateUploadMediaUrlUseCase {
    async fn execute(
        &self,
        _media_command: CreateMediaCommand,
        _attachment_command: CreateAttachmentCommand,
    ) -> Result<CreateMediaResult, CreateUrlError> {
        unimplemented!("StubCreateUploadMediaUrlUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubDeleteMediaUseCase;

#[async_trait]
impl DeleteMediaUseCase for StubDeleteMediaUseCase {
    async fn execute(&self, _owner: UserId, _media_id: Uuid) -> Result<(), DeleteMediaError> {
        unimplemented!("StubDeleteMediaUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubGetMediaUseCase;

#[async_trait]
impl GetMediaUseCase for StubGetMediaUseCase {
    async fn execute(&self, _owner: UserId, _media_id: Uuid) -> Result<MediaDetail, GetMediaError> {
        unimplemented!("StubGetMediaUseCase not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubGetVariantReadUrlService;

#[async_trait]
impl GetVariantReadUrlUseCase for StubGetVariantReadUrlService {
    async fn execute(&self, _command: GetUrlCommand) -> Result<GetUrlResult, GetReadUrlError> {
        unimplemented!()
    }
}

pub struct StubListMediaUseCase;

#[async_trait]
impl ListMediaUseCase for StubListMediaUseCase {
    async fn execute(&self, _command: ListMediaCommand) -> Result<Vec<MediaItem>, ListMediaError> {
        unimplemented!()
    }
}

// ============================================================================
// Blog stubs
//
// Every method panics: a test that reaches one has not configured the use case
// it actually exercises, and a silent default would hide that.
// ============================================================================

macro_rules! blog_stub {
    ($name:ident) => {
        #[derive(Clone, Default)]
        pub struct $name;
    };
}

blog_stub!(StubCreateBlogPost);
blog_stub!(StubGetBlogPosts);
blog_stub!(StubGetPublicBlogPosts);
blog_stub!(StubGetSingleBlogPost);
blog_stub!(StubGetPublicBlogPost);
blog_stub!(StubPatchBlogPost);
blog_stub!(StubArchiveBlogPost);
blog_stub!(StubRestoreBlogPost);
blog_stub!(StubHardDeleteBlogPost);
blog_stub!(StubAttachBlogPostTopic);
blog_stub!(StubDetachBlogPostTopic);
blog_stub!(StubClearBlogPostTopics);
blog_stub!(StubGetBlogPostTopics);

#[async_trait]
impl CreateBlogPostUseCase for StubCreateBlogPost {
    async fn execute(&self, _c: CreateBlogPostCommand) -> Result<BlogPost, CreateBlogPostError> {
        unimplemented!("StubCreateBlogPost not configured for this test")
    }
}

#[async_trait]
impl GetBlogPostsUseCase for StubGetBlogPosts {
    async fn execute(
        &self,
        _o: UserId,
        _f: BlogPostListFilter,
        _s: BlogPostSort,
        _p: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
        unimplemented!("StubGetBlogPosts not configured for this test")
    }
}

#[async_trait]
impl GetPublicBlogPostsUseCase for StubGetPublicBlogPosts {
    async fn execute(
        &self,
        _o: UserId,
        _f: BlogPostListFilter,
        _s: BlogPostSort,
        _p: BlogPageRequest,
    ) -> Result<BlogPageResult<BlogPostCard>, GetBlogPostsError> {
        unimplemented!("StubGetPublicBlogPosts not configured for this test")
    }
}

#[async_trait]
impl GetSingleBlogPostUseCase for StubGetSingleBlogPost {
    async fn execute(&self, _o: UserId, _p: Uuid) -> Result<BlogPostView, GetBlogPostError> {
        unimplemented!("StubGetSingleBlogPost not configured for this test")
    }
}

#[async_trait]
impl GetPublicBlogPostUseCase for StubGetPublicBlogPost {
    async fn execute(&self, _o: UserId, _s: &str) -> Result<BlogPostView, GetBlogPostError> {
        unimplemented!("StubGetPublicBlogPost not configured for this test")
    }
}

#[async_trait]
impl PatchBlogPostUseCase for StubPatchBlogPost {
    async fn execute(
        &self,
        _o: UserId,
        _p: Uuid,
        _d: PatchBlogPostData,
    ) -> Result<BlogPost, PatchBlogPostError> {
        unimplemented!("StubPatchBlogPost not configured for this test")
    }
}

#[async_trait]
impl ArchiveBlogPostUseCase for StubArchiveBlogPost {
    async fn execute(&self, _o: UserId, _p: Uuid) -> Result<(), ArchiveBlogPostError> {
        unimplemented!("StubArchiveBlogPost not configured for this test")
    }
}

#[async_trait]
impl RestoreBlogPostUseCase for StubRestoreBlogPost {
    async fn execute(&self, _o: UserId, _p: Uuid) -> Result<(), ArchiveBlogPostError> {
        unimplemented!("StubRestoreBlogPost not configured for this test")
    }
}

#[async_trait]
impl HardDeleteBlogPostUseCase for StubHardDeleteBlogPost {
    async fn execute(&self, _o: UserId, _p: Uuid) -> Result<(), ArchiveBlogPostError> {
        unimplemented!("StubHardDeleteBlogPost not configured for this test")
    }
}

#[async_trait]
impl AttachBlogPostTopicUseCase for StubAttachBlogPostTopic {
    async fn execute(&self, _o: UserId, _p: Uuid, _t: Uuid) -> Result<(), BlogPostTopicError> {
        unimplemented!("StubAttachBlogPostTopic not configured for this test")
    }
}

#[async_trait]
impl DetachBlogPostTopicUseCase for StubDetachBlogPostTopic {
    async fn execute(&self, _o: UserId, _p: Uuid, _t: Uuid) -> Result<(), BlogPostTopicError> {
        unimplemented!("StubDetachBlogPostTopic not configured for this test")
    }
}

#[async_trait]
impl ClearBlogPostTopicsUseCase for StubClearBlogPostTopics {
    async fn execute(&self, _o: UserId, _p: Uuid) -> Result<(), BlogPostTopicError> {
        unimplemented!("StubClearBlogPostTopics not configured for this test")
    }
}

#[async_trait]
impl GetBlogPostTopicsUseCase for StubGetBlogPostTopics {
    async fn execute(&self, _o: UserId, _p: Uuid) -> Result<Vec<BlogPostTopic>, GetBlogPostError> {
        unimplemented!("StubGetBlogPostTopics not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubRequestPasswordReset;

#[async_trait]
impl IRequestPasswordResetUseCase for StubRequestPasswordReset {
    async fn execute(&self, _email: &str) -> Result<(), RequestPasswordResetError> {
        unimplemented!("StubRequestPasswordReset not configured for this test")
    }
}

#[derive(Clone, Default)]
pub struct StubResetPassword;

#[async_trait]
impl IResetPasswordUseCase for StubResetPassword {
    async fn execute(&self, _token: &str, _password: &str) -> Result<(), ResetPasswordError> {
        unimplemented!("StubResetPassword not configured for this test")
    }
}

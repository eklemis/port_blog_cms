use crate::auth::application::helpers::UserIdentityResolver;
use crate::auth::application::orchestrator::user_registration::UserRegistrationOrchestrator;
use crate::auth::application::use_cases::fetch_profile::FetchUserProfileUseCase;
use crate::auth::application::use_cases::refresh_token::IRefreshTokenUseCase;
use crate::auth::application::use_cases::request_password_reset::IRequestPasswordResetUseCase;
use crate::auth::application::use_cases::resend_verification_email::IResendVerificationEmailUseCase;
use crate::auth::application::use_cases::reset_password::IResetPasswordUseCase;
use crate::auth::application::use_cases::soft_delete_user::ISoftDeleteUserUseCase;
use crate::auth::application::use_cases::update_profile::UpdateUserProfileUseCase;
use crate::auth::application::use_cases::{
    login_user::ILoginUserUseCase, logout_user::ILogoutUseCase,
    verify_user_email::IVerifyUserEmailUseCase,
};
use crate::blog::application::blog_use_cases::BlogUseCases;
use crate::blog::application::ports::incoming::use_cases::{
    ArchiveBlogPostUseCase, AttachBlogPostTopicUseCase, ClearBlogPostTopicsUseCase,
    CreateBlogPostUseCase, DetachBlogPostTopicUseCase, GetBlogPostTopicsUseCase,
    GetBlogPostsUseCase, GetPublicBlogPostUseCase, GetPublicBlogPostsUseCase,
    GetSingleBlogPostUseCase, HardDeleteBlogPostUseCase, PatchBlogPostUseCase,
    RestoreBlogPostUseCase,
};
use crate::cv::application::use_cases::create_cv::ICreateCVUseCase;
use crate::cv::application::use_cases::fetch_cv_by_id::IFetchCVByIdUseCase;
use crate::cv::application::use_cases::fetch_user_cvs::IFetchCVUseCase;
use crate::cv::application::use_cases::get_public_single_cv::GetPublicSingleCvUseCase;
use crate::cv::application::use_cases::hard_delete_cv::HardDeleteCvUseCase;
use crate::cv::application::use_cases::patch_cv::IPatchCVUseCase;
use crate::cv::application::use_cases::restore_cv::RestoreDeletedCvUseCase;
use crate::cv::application::use_cases::soft_delete_cv::SoftDeleteCvUseCase;
use crate::cv::application::use_cases::update_cv::IUpdateCVUseCase;
use crate::modules::project::application::ports::incoming::use_cases::CreateProjectUseCase;
use crate::modules::project::application::project_use_cases::ProjectUseCases;
use crate::multimedia::application::domain::policies::upload_policy::UploadPolicy;
use crate::multimedia::application::media_use_cases::MultimediaUseCases;
use crate::multimedia::application::ports::incoming::use_cases::{
    CreateUploadMediaUrlUseCase, DeleteMediaUseCase, GetMediaUseCase, GetVariantReadUrlUseCase,
    ListMediaUseCase,
};
use crate::project::application::ports::incoming::use_cases::{
    GetProjectsUseCase, GetPublicSingleProjectUseCase, GetSingleProjectUseCase, PatchProjectUseCase,
};
use crate::tests::support::stubs::*;
use crate::topic::application::ports::incoming::use_cases::{
    CreateTopicUseCase, GetTopicsUseCase, SoftDeleteTopicUseCase,
};
use crate::AppState;
use actix_web::web;
use std::sync::Arc;

pub struct TestAppStateBuilder {
    fetch_cv: Option<Arc<dyn IFetchCVUseCase + Send + Sync>>,
    fetch_cv_by_id: Option<Arc<dyn IFetchCVByIdUseCase + Send + Sync>>,
    get_public_single_cv_use_case: Option<Arc<dyn GetPublicSingleCvUseCase + Send + Sync>>,
    create_cv: Option<Arc<dyn ICreateCVUseCase + Send + Sync>>,
    update_cv: Option<Arc<dyn IUpdateCVUseCase + Send + Sync>>,
    patch_cv: Option<Arc<dyn IPatchCVUseCase + Send + Sync>>,
    register_user: Option<Arc<UserRegistrationOrchestrator>>,
    verify_user_email: Option<Arc<dyn IVerifyUserEmailUseCase + Send + Sync>>,
    login_user: Option<Arc<dyn ILoginUserUseCase + Send + Sync>>,
    refresh_token: Option<Arc<dyn IRefreshTokenUseCase + Send + Sync>>,
    request_password_reset: Option<Arc<dyn IRequestPasswordResetUseCase + Send + Sync>>,
    resend_verification_email: Option<Arc<dyn IResendVerificationEmailUseCase + Send + Sync>>,
    reset_password: Option<Arc<dyn IResetPasswordUseCase + Send + Sync>>,
    logout_user: Option<Arc<dyn ILogoutUseCase + Send + Sync>>,
    soft_delete_user: Option<Arc<dyn ISoftDeleteUserUseCase + Send + Sync>>,
    fetch_user_profile: Option<Arc<dyn FetchUserProfileUseCase + Send + Sync>>,
    update_user_profile: Option<Arc<dyn UpdateUserProfileUseCase + Send + Sync>>,
    hard_delete_cv: Option<Arc<dyn HardDeleteCvUseCase + Send + Sync>>,
    soft_delete_cv: Option<Arc<dyn SoftDeleteCvUseCase + Send + Sync>>,
    restore_cv: Option<Arc<dyn RestoreDeletedCvUseCase + Send + Sync>>,
    create_topic: Option<Arc<dyn CreateTopicUseCase + Send + Sync>>,
    get_topics: Option<Arc<dyn GetTopicsUseCase + Send + Sync>>,
    soft_delete_topic: Option<Arc<dyn SoftDeleteTopicUseCase + Send + Sync>>,
    blog: Option<BlogUseCases>,
    project: Option<ProjectUseCases>,
    multimedia: Option<MultimediaUseCases>,
    user_identity_resolver: Option<UserIdentityResolver>,
}

pub fn default_test_user_registration_orchestrator() -> Arc<UserRegistrationOrchestrator> {
    let create_user = Arc::new(StubCreateUserUseCase);
    let email_notifier = Arc::new(StubUserEmailNotifier);

    Arc::new(UserRegistrationOrchestrator::new(
        create_user,
        Arc::new(StubTokenProvider),
        email_notifier,
    ))
}

impl Default for TestAppStateBuilder {
    fn default() -> Self {
        let user_identity_resolver = UserIdentityResolver::new(Arc::new(DummyUserQuery));
        Self {
            fetch_cv: Some(Arc::new(StubFetchCVUseCase)),
            fetch_cv_by_id: Some(Arc::new(StubFetchCVByIdUseCase)),
            get_public_single_cv_use_case: Some(StubGetPublicSingleCvUseCase::not_found()),
            create_cv: Some(Arc::new(StubCreateCVUseCase)),
            update_cv: Some(Arc::new(StubUpdateCVUseCase)),
            patch_cv: Some(Arc::new(StubPatchCVUseCase)),
            register_user: Some(default_test_user_registration_orchestrator()),
            verify_user_email: Some(Arc::new(StubVerifyUserEmailUseCase)),
            login_user: Some(Arc::new(StubLoginUserUseCase)),
            refresh_token: Some(Arc::new(StubRefreshTokenUseCase)),
            request_password_reset: Some(Arc::new(StubRequestPasswordReset)),
            resend_verification_email: Some(Arc::new(StubResendVerificationEmail)),
            reset_password: Some(Arc::new(StubResetPassword)),
            logout_user: Some(Arc::new(StubLogoutUserUseCase)),
            soft_delete_user: Some(Arc::new(StubSoftDeleteUserUseCase)),
            fetch_user_profile: Some(Arc::new(StubFetchUserProfileUseCase)),
            update_user_profile: Some(Arc::new(StubUpdateUserProfileUseCase)),
            hard_delete_cv: Some(Arc::new(StubHardDeleteCvUseCase)),
            soft_delete_cv: Some(Arc::new(StubSoftDeleteCv)),
            restore_cv: Some(Arc::new(StubRestoreDeletedCv)),
            create_topic: Some(Arc::new(StubCreateTopicUseCase)),
            get_topics: Some(Arc::new(StubGetTopicsUseCase::success(vec![]))),
            soft_delete_topic: Some(Arc::new(StubSoftDeleteTopicUseCase)),
            blog: Some(BlogUseCases {
                slug_available: Arc::new(StubSlugAvailable),
                create: Arc::new(StubCreateBlogPost),
                list: Arc::new(StubGetBlogPosts),
                list_public: Arc::new(StubGetPublicBlogPosts),
                get_single: Arc::new(StubGetSingleBlogPost),
                get_public: Arc::new(StubGetPublicBlogPost),
                patch: Arc::new(StubPatchBlogPost),
                archive: Arc::new(StubArchiveBlogPost),
                restore: Arc::new(StubRestoreBlogPost),
                hard_delete: Arc::new(StubHardDeleteBlogPost),
                attach_topic: Arc::new(StubAttachBlogPostTopic),
                detach_topic: Arc::new(StubDetachBlogPostTopic),
                clear_topics: Arc::new(StubClearBlogPostTopics),
                get_topics: Arc::new(StubGetBlogPostTopics),
            }),
            project: Some(ProjectUseCases {
                slug_available: Arc::new(StubProjectSlugAvailable),
                restore: Arc::new(StubRestoreProject),
                create: Arc::new(StubCreateProjectUseCase::repo_error(
                    "not used in this test",
                )),
                get_list: Arc::new(DefaultStubGetProjectsUseCase),
                get_single: Arc::new(StubGetSingleProjectUseCase::not_found()),
                get_public_single: Arc::new(StubGetPublicSingleProjectUseCase::not_found()),
                patch: Arc::new(DefaultStubPatchProjectUseCase),
                add_topic: Arc::new(StubAddProjectTopicUseCase),
                get_topics: Arc::new(StubGetProjectTopicsUseCase),
                remove_topic: Arc::new(StubRemoveProjectTopicUseCase),
                clear_topics: Arc::new(StubClearProjectTopicsUseCase),
                hard_delete: Arc::new(StubHardDeleteProjectUseCase),
                soft_delete: Arc::new(StubSoftDeleteProjectUseCase),
            }),
            multimedia: Some(MultimediaUseCases {
                get_public_variant_url: Arc::new(StubGetPublicVariantUrl),
                patch_media: Arc::new(StubMediaLifecycle),
                restore_media: Arc::new(StubMediaLifecycle),
                hard_delete_media: Arc::new(StubMediaLifecycle),
                get_media_usage: Arc::new(StubMediaLifecycle),
                get_media_statuses: Arc::new(StubMediaLifecycle),
                create_signed_post_url: Arc::new(StubCreateUploadMediaUrlUseCase),
                create_signed_get_url: Arc::new(StubGetVariantReadUrlService),
                list_media: Arc::new(StubListMediaUseCase),
                delete_media: Arc::new(StubDeleteMediaUseCase),
                get_media: Arc::new(StubGetMediaUseCase),
            }),
            user_identity_resolver: Some(user_identity_resolver),
        }
    }
}

impl TestAppStateBuilder {
    pub fn with_blog_create(mut self, uc: impl CreateBlogPostUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.create = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_list(mut self, uc: impl GetBlogPostsUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.list = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_list_public(mut self, uc: impl GetPublicBlogPostsUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.list_public = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_get_single(mut self, uc: impl GetSingleBlogPostUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.get_single = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_get_public(mut self, uc: impl GetPublicBlogPostUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.get_public = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_patch(mut self, uc: impl PatchBlogPostUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.patch = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_archive(mut self, uc: impl ArchiveBlogPostUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.archive = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_restore(mut self, uc: impl RestoreBlogPostUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.restore = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_hard_delete(mut self, uc: impl HardDeleteBlogPostUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.hard_delete = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_attach_topic(mut self, uc: impl AttachBlogPostTopicUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.attach_topic = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_detach_topic(mut self, uc: impl DetachBlogPostTopicUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.detach_topic = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_clear_topics(mut self, uc: impl ClearBlogPostTopicsUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.clear_topics = std::sync::Arc::new(uc);
        self
    }

    pub fn with_blog_get_topics(mut self, uc: impl GetBlogPostTopicsUseCase + 'static) -> Self {
        let blog = self
            .blog
            .as_mut()
            .expect("Blog use cases must be initialized");
        blog.get_topics = std::sync::Arc::new(uc);
        self
    }

    /// Overrides the verification-resend use case.
    pub fn with_resend_verification_email(
        mut self,
        uc: impl IResendVerificationEmailUseCase + 'static,
    ) -> Self {
        self.resend_verification_email = Some(Arc::new(uc));
        self
    }

    pub fn with_request_password_reset(
        mut self,
        uc: impl IRequestPasswordResetUseCase + 'static,
    ) -> Self {
        self.request_password_reset = Some(Arc::new(uc));
        self
    }

    pub fn with_reset_password(mut self, uc: impl IResetPasswordUseCase + 'static) -> Self {
        self.reset_password = Some(Arc::new(uc));
        self
    }

    pub fn with_create_cv(mut self, uc: impl ICreateCVUseCase + 'static) -> Self {
        self.create_cv = Some(Arc::new(uc));
        self
    }
    pub fn with_fetch_cv(mut self, uc: impl IFetchCVUseCase + 'static) -> Self {
        self.fetch_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_fetch_cv_by_id(mut self, uc: impl IFetchCVByIdUseCase + 'static) -> Self {
        self.fetch_cv_by_id = Some(Arc::new(uc));
        self
    }

    // In TestAppStateBuilder
    pub fn with_register_user_orchestrator(
        mut self,
        orchestrator: Arc<UserRegistrationOrchestrator>,
    ) -> Self {
        self.register_user = Some(orchestrator);
        self
    }

    pub fn with_update_cv(mut self, uc: Arc<dyn IUpdateCVUseCase + Send + Sync>) -> Self {
        self.update_cv = Some(uc);
        self
    }

    pub fn with_patch_cv(mut self, uc: impl IPatchCVUseCase + 'static) -> Self {
        self.patch_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_login_user(mut self, uc: impl ILoginUserUseCase + 'static) -> Self {
        self.login_user = Some(Arc::new(uc));
        self
    }

    pub fn with_verify_user_email(mut self, uc: impl IVerifyUserEmailUseCase + 'static) -> Self {
        self.verify_user_email = Some(Arc::new(uc));
        self
    }

    pub fn with_refresh_token(mut self, uc: impl IRefreshTokenUseCase + 'static) -> Self {
        self.refresh_token = Some(Arc::new(uc));
        self
    }

    pub fn with_logout_user(mut self, uc: impl ILogoutUseCase + 'static) -> Self {
        self.logout_user = Some(Arc::new(uc));
        self
    }

    pub fn with_soft_delete_user(mut self, uc: impl ISoftDeleteUserUseCase + 'static) -> Self {
        self.soft_delete_user = Some(Arc::new(uc));
        self
    }

    pub fn with_fetch_user_profile(mut self, uc: impl FetchUserProfileUseCase + 'static) -> Self {
        self.fetch_user_profile = Some(Arc::new(uc));
        self
    }

    pub fn with_update_user_profile(mut self, uc: impl UpdateUserProfileUseCase + 'static) -> Self {
        self.update_user_profile = Some(Arc::new(uc));
        self
    }
    pub fn with_soft_delete_cv(mut self, uc: impl SoftDeleteCvUseCase + 'static) -> Self {
        self.soft_delete_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_restore_cv(mut self, uc: impl RestoreDeletedCvUseCase + 'static) -> Self {
        self.restore_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_hard_delete_cv(mut self, uc: impl HardDeleteCvUseCase + 'static) -> Self {
        self.hard_delete_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_create_topic(mut self, uc: impl CreateTopicUseCase + 'static) -> Self {
        self.create_topic = Some(Arc::new(uc));
        self
    }

    pub fn with_get_topics(mut self, uc: impl GetTopicsUseCase + 'static) -> Self {
        self.get_topics = Some(Arc::new(uc));
        self
    }

    pub fn with_soft_delete_topic(mut self, uc: impl SoftDeleteTopicUseCase + 'static) -> Self {
        self.soft_delete_topic = Some(Arc::new(uc));
        self
    }
    pub fn with_create_project_use_case(mut self, uc: impl CreateProjectUseCase + 'static) -> Self {
        if let Some(mut p) = self.project.take() {
            p.create = Arc::new(uc);
            self.project = Some(p);
        }
        self
    }
    pub fn with_get_projects(mut self, uc: impl GetProjectsUseCase + 'static) -> Self {
        if let Some(mut p) = self.project.take() {
            p.get_list = Arc::new(uc);
            self.project = Some(p);
        }
        self
    }
    pub fn with_get_single_project(mut self, uc: impl GetSingleProjectUseCase + 'static) -> Self {
        // ProjectUseCases is guaranteed to exist from Default
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.get_single = Arc::new(uc);
        self
    }
    pub fn with_patch_project(mut self, uc: impl PatchProjectUseCase + 'static) -> Self {
        // ProjectUseCases is guaranteed to exist from Default
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.patch = Arc::new(uc);
        self
    }
    pub fn with_user_identity_resolver(
        mut self,
        resolver: crate::auth::application::helpers::UserIdentityResolver,
    ) -> Self {
        self.user_identity_resolver = Some(resolver);
        self
    }

    pub fn with_get_public_single_project(
        mut self,
        uc: impl GetPublicSingleProjectUseCase + 'static,
    ) -> Self {
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.get_public_single = Arc::new(uc);
        self
    }

    pub fn with_get_public_single_cv(
        mut self,
        uc: Arc<dyn GetPublicSingleCvUseCase + Send + Sync>,
    ) -> Self {
        self.get_public_single_cv_use_case = Some(uc);
        self
    }
    pub fn with_add_project_topic<U>(mut self, uc: U) -> Self
    where
        U: crate::modules::project::application::ports::incoming::use_cases::AddProjectTopicUseCase
            + Send
            + Sync
            + 'static,
    {
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.add_topic = std::sync::Arc::new(uc);
        self
    }
    pub fn with_remove_project_topic<U>(mut self, uc: U) -> Self
    where
        U: crate::modules::project::application::ports::incoming::use_cases::RemoveProjectTopicUseCase
            + Send
            + Sync
            + 'static,
    {
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.remove_topic = std::sync::Arc::new(uc);
        self
    }
    pub fn with_clear_project_topics<U>(mut self, uc: U) -> Self
    where
        U: crate::modules::project::application::ports::incoming::use_cases::ClearProjectTopicsUseCase
            + Send
            + Sync
            + 'static,
    {
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.clear_topics = std::sync::Arc::new(uc);
        self
    }
    pub fn with_get_project_topics<U>(mut self, uc: U) -> Self
    where
        U: crate::modules::project::application::ports::incoming::use_cases::GetProjectTopicsUseCase
            + Send
            + Sync
            + 'static,
    {
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.get_topics = std::sync::Arc::new(uc);
        self
    }
    pub fn with_hard_delete_project<U>(mut self, uc: U) -> Self
    where
        U: crate::modules::project::application::ports::incoming::use_cases::HardDeleteProjectUseCase
            + Send
            + Sync
            + 'static,
    {
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.hard_delete = std::sync::Arc::new(uc);
        self
    }
    pub fn with_soft_delete_project<U>(mut self, uc: U) -> Self
    where
        U: crate::modules::project::application::ports::incoming::use_cases::SoftDeleteProjectUseCase
            + Send
            + Sync
            + 'static,
    {
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.soft_delete = std::sync::Arc::new(uc);
        self
    }
    pub fn with_get_media(mut self, uc: impl GetMediaUseCase + 'static) -> Self {
        let multimedia = self
            .multimedia
            .as_mut()
            .expect("Multimedia use cases must be initialized");
        multimedia.get_media = std::sync::Arc::new(uc);
        self
    }

    pub fn with_delete_media(mut self, uc: impl DeleteMediaUseCase + 'static) -> Self {
        let multimedia = self
            .multimedia
            .as_mut()
            .expect("Multimedia use cases must be initialized");
        multimedia.delete_media = std::sync::Arc::new(uc);
        self
    }

    pub fn with_create_upload_media_url(
        mut self,
        uc: impl CreateUploadMediaUrlUseCase + 'static,
    ) -> Self {
        let multimedia = self
            .multimedia
            .as_mut()
            .expect("Multimedia use cases must be initialized");

        multimedia.create_signed_post_url = Arc::new(uc);
        self
    }
    pub fn with_create_signed_get_url(
        mut self,
        uc: impl GetVariantReadUrlUseCase + 'static,
    ) -> Self {
        let multimedia = self
            .multimedia
            .as_mut()
            .expect("Multimedia use cases must be initialized");

        multimedia.create_signed_get_url = Arc::new(uc);
        self
    }
    pub fn with_list_media(mut self, uc: impl ListMediaUseCase + 'static) -> Self {
        let multimedia = self
            .multimedia
            .as_mut()
            .expect("Multimedia use cases must be initialized");

        multimedia.list_media = Arc::new(uc);
        self
    }
    pub fn build(self) -> web::Data<AppState> {
        web::Data::new(AppState {
            fetch_cv_use_case: self.fetch_cv.unwrap(),
            fetch_cv_by_id_use_case: self.fetch_cv_by_id.unwrap(),
            get_public_single_cv_use_case: self
                .get_public_single_cv_use_case
                .expect("get_public_single_cv_use_case not set"),
            create_cv_use_case: self.create_cv.unwrap(),
            update_cv_use_case: self.update_cv.unwrap(),
            patch_cv_use_case: self.patch_cv.unwrap(),
            register_user_orchestrator: self.register_user.unwrap(),
            verify_user_email_use_case: self.verify_user_email.unwrap(),
            login_user_use_case: self.login_user.unwrap(),
            refresh_token_use_case: self.refresh_token.unwrap(),
            request_password_reset_use_case: self.request_password_reset.unwrap(),
            resend_verification_email_use_case: self.resend_verification_email.unwrap(),
            reset_password_use_case: self.reset_password.unwrap(),
            logout_user_use_case: self.logout_user.unwrap(),
            soft_delete_user_use_case: self.soft_delete_user.unwrap(),
            fetch_user_profile_use_case: self.fetch_user_profile.unwrap(),
            update_user_profile_use_case: self.update_user_profile.unwrap(),
            hard_delete_cv_use_case: self.hard_delete_cv.unwrap(),
            soft_delete_cv_use_case: self.soft_delete_cv.unwrap(),
            restore_cv_use_case: self.restore_cv.unwrap(),
            create_topic_use_case: self.create_topic.unwrap(),
            get_topics_use_case: self.get_topics.unwrap(),
            get_topic_usage_use_case: Arc::new(StubGetTopicUsage),
            patch_topic_use_case: Arc::new(StubPatchTopic),
            soft_delete_topic_use_case: self.soft_delete_topic.unwrap(),
            blog: self.blog.unwrap(),
            project: self.project.unwrap(),
            multimedia: self.multimedia.unwrap(),
            user_identity_resolver: self.user_identity_resolver.unwrap(),
            multimedia_upload_policy: UploadPolicy::from_env(),
        })
    }
}

use crate::auth::application::helpers::UserIdentityResolver;
use crate::auth::application::orchestrator::user_registration::UserRegistrationOrchestrator;
use crate::auth::application::use_cases::fetch_profile::FetchUserProfileUseCase;
use crate::auth::application::use_cases::refresh_token::IRefreshTokenUseCase;
use crate::auth::application::use_cases::soft_delete_user::ISoftDeleteUserUseCase;
use crate::auth::application::use_cases::update_profile::UpdateUserProfileUseCase;
use crate::auth::application::use_cases::{
    login_user::ILoginUserUseCase, logout_user::ILogoutUseCase,
    verify_user_email::IVerifyUserEmailUseCase,
};
use crate::cv::application::use_cases::create_cv::ICreateCVUseCase;
use crate::cv::application::use_cases::fetch_cv_by_id::IFetchCVByIdUseCase;
use crate::cv::application::use_cases::fetch_user_cvs::IFetchCVUseCase;
use crate::cv::application::use_cases::get_public_single_cv::GetPublicSingleCvUseCase;
use crate::cv::application::use_cases::hard_delete_cv::HardDeleteCvUseCase;
use crate::cv::application::use_cases::patch_cv::IPatchCVUseCase;
use crate::cv::application::use_cases::update_cv::IUpdateCVUseCase;
use crate::modules::project::application::ports::incoming::use_cases::CreateProjectUseCase;
use crate::modules::project::application::project_use_cases::ProjectUseCases;
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
    logout_user: Option<Arc<dyn ILogoutUseCase + Send + Sync>>,
    soft_delete_user: Option<Arc<dyn ISoftDeleteUserUseCase + Send + Sync>>,
    fetch_user_profile: Option<Arc<dyn FetchUserProfileUseCase + Send + Sync>>,
    update_user_profile: Option<Arc<dyn UpdateUserProfileUseCase + Send + Sync>>,
    hard_delete_cv: Option<Arc<dyn HardDeleteCvUseCase + Send + Sync>>,
    create_topic: Option<Arc<dyn CreateTopicUseCase + Send + Sync>>,
    get_topics: Option<Arc<dyn GetTopicsUseCase + Send + Sync>>,
    soft_delete_topic: Option<Arc<dyn SoftDeleteTopicUseCase + Send + Sync>>,
    project: Option<ProjectUseCases>,
    user_identity_resolver: Option<UserIdentityResolver>,
}

pub fn default_test_user_registration_orchestrator() -> Arc<UserRegistrationOrchestrator> {
    let create_user = Arc::new(StubCreateUserUseCase);
    let email_notifier = Arc::new(StubUserEmailNotifier);

    Arc::new(UserRegistrationOrchestrator::new(
        create_user,
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
            logout_user: Some(Arc::new(StubLogoutUserUseCase)),
            soft_delete_user: Some(Arc::new(StubSoftDeleteUserUseCase)),
            fetch_user_profile: Some(Arc::new(StubFetchUserProfileUseCase)),
            update_user_profile: Some(Arc::new(StubUpdateUserProfileUseCase)),
            hard_delete_cv: Some(Arc::new(StubHardDeleteCvUseCase)),
            create_topic: Some(Arc::new(StubCreateTopicUseCase)),
            get_topics: Some(Arc::new(StubGetTopicsUseCase::success(vec![]))),
            soft_delete_topic: Some(Arc::new(StubSoftDeleteTopicUseCase)),
            project: Some(ProjectUseCases {
                create: Arc::new(StubCreateProjectUseCase::repo_error(
                    "not used in this test",
                )),
                get_list: Arc::new(DefaultStubGetProjectsUseCase),
                get_single: Arc::new(StubGetSingleProjectUseCase::not_found()),
                get_public_single: Arc::new(StubGetPublicSingleProjectUseCase::not_found()),
                patch: Arc::new(DefaultStubPatchProjectUseCase),
                add_topic: Arc::new(StubAddProjectTopicUseCase),
                remove_topic: Arc::new(StubRemoveProjectTopicUseCase),
                clear_topics: Arc::new(StubClearProjectTopicsUseCase),
            }),
            user_identity_resolver: Some(user_identity_resolver),
        }
    }
}

impl TestAppStateBuilder {
    pub fn with_create_cv(mut self, uc: impl ICreateCVUseCase + Send + Sync + 'static) -> Self {
        self.create_cv = Some(Arc::new(uc));
        self
    }
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

    pub fn with_patch_cv(mut self, uc: impl IPatchCVUseCase + Send + Sync + 'static) -> Self {
        self.patch_cv = Some(Arc::new(uc));
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

    pub fn with_soft_delete_user(
        mut self,
        uc: impl ISoftDeleteUserUseCase + Send + Sync + 'static,
    ) -> Self {
        self.soft_delete_user = Some(Arc::new(uc));
        self
    }

    pub fn with_fetch_user_profile(
        mut self,
        uc: impl FetchUserProfileUseCase + Send + Sync + 'static,
    ) -> Self {
        self.fetch_user_profile = Some(Arc::new(uc));
        self
    }

    pub fn with_update_user_profile(
        mut self,
        uc: impl UpdateUserProfileUseCase + Send + Sync + 'static,
    ) -> Self {
        self.update_user_profile = Some(Arc::new(uc));
        self
    }
    pub fn with_hard_delete_cv(
        mut self,
        uc: impl HardDeleteCvUseCase + Send + Sync + 'static,
    ) -> Self {
        self.hard_delete_cv = Some(Arc::new(uc));
        self
    }

    pub fn with_create_topic(
        mut self,
        uc: impl CreateTopicUseCase + Send + Sync + 'static,
    ) -> Self {
        self.create_topic = Some(Arc::new(uc));
        self
    }

    pub fn with_get_topics(mut self, uc: impl GetTopicsUseCase + Send + Sync + 'static) -> Self {
        self.get_topics = Some(Arc::new(uc));
        self
    }

    pub fn with_soft_delete_topic(
        mut self,
        uc: impl SoftDeleteTopicUseCase + Send + Sync + 'static,
    ) -> Self {
        self.soft_delete_topic = Some(Arc::new(uc));
        self
    }
    pub fn with_create_project_use_case(
        mut self,
        uc: impl CreateProjectUseCase + Send + Sync + 'static,
    ) -> Self {
        if let Some(mut p) = self.project.take() {
            p.create = Arc::new(uc);
            self.project = Some(p);
        }
        self
    }
    pub fn with_get_projects(
        mut self,
        uc: impl GetProjectsUseCase + Send + Sync + 'static,
    ) -> Self {
        if let Some(mut p) = self.project.take() {
            p.get_list = Arc::new(uc);
            self.project = Some(p);
        }
        self
    }
    pub fn with_get_single_project(
        mut self,
        uc: impl GetSingleProjectUseCase + Send + Sync + 'static,
    ) -> Self {
        // ProjectUseCases is guaranteed to exist from Default
        let project = self
            .project
            .as_mut()
            .expect("Project use cases must be initialized");

        project.get_single = Arc::new(uc);
        self
    }
    pub fn with_patch_project(
        mut self,
        uc: impl PatchProjectUseCase + Send + Sync + 'static,
    ) -> Self {
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
        uc: impl GetPublicSingleProjectUseCase + Send + Sync + 'static,
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
            logout_user_use_case: self.logout_user.unwrap(),
            soft_delete_user_use_case: self.soft_delete_user.unwrap(),
            fetch_user_profile_use_case: self.fetch_user_profile.unwrap(),
            update_user_profile_use_case: self.update_user_profile.unwrap(),
            hard_delete_cv_use_case: self.hard_delete_cv.unwrap(),
            create_topic_use_case: self.create_topic.unwrap(),
            get_topics_use_case: self.get_topics.unwrap(),
            soft_delete_topic_use_case: self.soft_delete_topic.unwrap(),
            project: self.project.unwrap(),
            user_identity_resolver: self.user_identity_resolver.unwrap(),
        })
    }
}

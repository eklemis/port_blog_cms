pub mod app_state_builder;
pub mod auth_helper;
pub mod project_test_fixtures;
pub mod stubs;

#[cfg(test)]
pub fn load_test_env() {
    dotenvy::from_filename(".env.test").ok();
}

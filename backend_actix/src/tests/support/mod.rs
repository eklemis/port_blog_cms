pub mod app_state_builder;
pub mod auth_helper;
pub mod stubs;

#[cfg(test)]
pub fn load_test_env() {
    dotenvy::from_filename(".env.test").ok();
}

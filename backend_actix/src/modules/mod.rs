/// Model-backed surfaces and the generation allowance that governs them.
pub mod ai;
/// Users, tokens, password hashing and sessions. Owns `UserId`, which every
/// other module uses.
pub mod auth;
/// Blog posts, their publication lifecycle and their topic links.
pub mod blog;
/// Job postings, applications, and what happened to them.
pub mod career;
/// CVs, including the public read views.
pub mod cv;
/// Verification and password-reset mail. A support module: no routes.
pub mod email;
/// Media uploads: signed URLs, variants and the upload policy.
pub mod multimedia;
/// Projects, their topic links and the public project views.
pub mod project;
/// Topics, scoped per user. Shared vocabulary for blog and project.
pub mod topic;

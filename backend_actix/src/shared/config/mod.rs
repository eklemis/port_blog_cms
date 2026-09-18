/// Reading the mail configuration, and refusing values that look fine and are
/// not — a link with no scheme, a bind address, a host that was set under the
/// wrong name.
pub mod mail;

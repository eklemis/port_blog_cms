# 0005 — `email` does not depend on `auth`

**Status:** Accepted
**Supersedes:** the "known structural issue" recorded in `ARCHITECTURE.md`
**Component:** `modules/email`, `modules/auth`

## Context

`auth` and `email` imported each other.

`auth → email` is the right direction: registration and password reset need to
send mail, and depending on `email`'s notifier ports rather than on SMTP is
exactly what ports are for.

The other direction was accidental. `email` imported two things from `auth`:

- `CreateUserOutput`, because `UserEmailNotifier::send_verification_email` was
  typed on it — a registration-shaped DTO;
- `TokenProvider`, because `UserEmailService` minted the verification token
  itself.

So the more generic module could not be read, moved or tested without the more
specific one. Rust permits cycles inside a crate, so this compiled and nothing
forced the issue.

The asymmetry that made the fix obvious: `send_password_reset_email` already
took an **already-minted** token as an argument. Only verification minted its
own. Two methods on the same service, doing the same job, built differently.

## Decision

**`email` owns its own input type and never mints tokens.** Verification now
works the way reset already did.

- A new `Recipient { email, username }`, owned by `email`, replaces
  `CreateUserOutput` on the port.
- Both notifier methods take `(&Recipient, token: &str)`. The caller mints.
- `UserEmailService` drops its `TokenProvider` generic and is now generic only
  over the transport: `UserEmailService<E: EmailSender>`.
- `UserRegistrationOrchestrator` holds the `TokenProvider` and mints the
  verification token before spawning the send.

## Consequences

`email` now imports nothing from any other module — a leaf, which is what a
generic notification module should be. The dependency graph has no cycles.

**Token-minting failures now surface from `register_user`.** Previously the
notifier minted inside a detached `tokio::spawn`, so a minting failure was
logged and lost. It is now a `UserRegistrationError::TokenGenerationFailed`
returned to the caller. This is a behaviour change, and an improvement: a
failure to mint is a server fault the caller should hear about, not a silent
non-delivery. Delivery failures remain fire-and-forget with retries, as before.

The test that covered "a token failure sends nothing" moved from
`email_service` to the orchestrator, following the responsibility.

Two costs, both accepted:

- The orchestrator gained a constructor argument, so every test that builds one
  needs a token provider. A shared `StubTokenProvider` lives in
  `tests/support/stubs.rs`.
- `Recipient` duplicates two fields that also exist on `CreateUserOutput`. That
  duplication is the point — it is what stops a change to `auth`'s registration
  DTO rippling into `email`'s templates.

## Alternatives considered

**Give the port its own type but leave `TokenProvider` alone.** This was the
original recommendation in `ARCHITECTURE.md`, on the grounds that minting is
genuinely `auth`'s job and a separate link-signing port would be a larger
change. It was wrong: it removes one of the two edges and leaves the cycle
intact. Moving the mint to the caller turned out to be smaller than the
link-signing port it was compared against, because the reset path already
worked that way.

**Move `email` into `auth`.** Rejected: the coupling is one DTO and one call,
not a reason to merge two coherent modules.

# Authentication

Bearer tokens in the `Authorization` header. **No cookies** — the API sets none
and reads none, so there is no CSRF surface and no `SameSite` behaviour to work
around, and storing the tokens is entirely the client's problem.

```
Authorization: Bearer <access_token>
```

## The flow

```
POST /api/auth/register   → 201, an email is sent
GET  /api/auth/email-verification/{token}   (from that email)
POST /api/auth/login      → 200 { access_token, refresh_token, user }
POST /api/auth/refresh    → 200 { access_token, refresh_token }
POST /api/auth/logout     → 200
```

**Login succeeds for an unverified account.** It returns real tokens. What an
unverified account cannot do is use most of the API — see
[Two levels of access](#two-levels-of-access) below. Do not gate your login
screen on verification; gate the workspace.

### Login

`POST /api/auth/login`, public. Body: `email`, `password` — both required.

```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOi…",
    "refresh_token": "eyJhbGciOi…",
    "user": { "id": "…", "username": "…", "email": "…", "is_verified": false }
  }
}
```

`401 INVALID_CREDENTIALS` for a wrong email or password — **the same code for
both**, deliberately, so the endpoint cannot be used to discover which accounts
exist.

### Refresh

`POST /api/auth/refresh`, public. Body: `refresh_token`, required. Returns a new
pair.

| Token | Lifetime | Env var |
| --- | --- | --- |
| Access | 30 minutes (1800s) | `JWT_ACCESS_EXPIRY` |
| Refresh | 7 days (604800s) | `JWT_REFRESH_EXPIRY` |

Those are the defaults; a deployment can change them, so **read expiry from the
token rather than hard-coding 30 minutes.**

Refresh on `401`, retry once, and send the user to login if the retry also
fails. Refreshing on a timer is fine too, but the `401` path has to exist
regardless — a token can be invalidated before it expires by a logout.

### Logout

`POST /api/auth/logout`. Body: `refresh_token`, **optional**.

It always answers `200`, including for a token that was already invalid. That is
intentional: logout is something a client does while tearing down state, and a
failure it cannot act on is worse than silence. Clear your tokens either way.

Supplying the `refresh_token` blacklists it server-side, which is what stops a
stolen refresh token outliving the session. **Send it.** Omitting it makes
logout purely client-side.

## Two levels of access

Two extractors guard the API, and the difference is visible to your users:

| Guard | Requires | Endpoints |
| --- | --- | --- |
| `AuthenticatedUser` | A valid token | `GET`/`PUT`/`DELETE /api/users/me` |
| `VerifiedUser` | A valid token **and** a verified email | everything else |

So an account that has registered but not clicked the email link can sign in,
read and edit its own profile, and delete itself — and nothing else. Every other
authenticated route answers `403`.

**Build for this state.** It is not an edge case; it is where every new user
starts. `GET /api/users/me` returns `is_verified`, and
`POST /api/auth/email-verification/resend` re-sends the link.

## Which failure means what

| Status | Code | What happened | What to do |
| --- | --- | --- | --- |
| 401 | `MISSING_AUTH_HEADER` | No `Authorization` header | Send to login |
| 401 | `INVALID_TOKEN` | Malformed, expired, or blacklisted | Refresh once, then login |
| 401 | `INVALID_TOKEN_TYPE` | An access token sent where a refresh one belongs, or vice versa | Fix the call |
| 401 | `INVALID_CREDENTIALS` | Wrong email or password | Show one message for both |
| 403 | `EMAIL_NOT_VERIFIED` | Valid token, unverified account | Prompt to verify; offer resend |
| 429 | `RATE_LIMITED` | Too many attempts | Back off; `Retry-After` is set |

**`401` and `403` mean different things here and should not share a handler.**
A `401` is "prove who you are again" and a refresh may fix it. A `403` on these
routes is "we know who you are, and this account has not verified its email" —
refreshing will produce the same result forever.

## Rate limiting

Auth endpoints are rate limited per caller. On `429`, honour `Retry-After`.

Two behaviours worth knowing, both recorded as decision records:

- **The limiter fails open when Redis is unreachable** — requests are allowed
  rather than refused ([ADR 0001](adr/0001-rate-limiter-fails-open.md)).
- **Callers are keyed on the left-most `X-Forwarded-For` entry**
  ([ADR 0002](adr/0002-rate-limit-keying-on-forwarded-for.md)).

## Password reset

```
POST /api/auth/password-reset          { email }        → 200, always
POST /api/auth/password-reset/{token}  { password }     → 200
```

The first **always answers `200`**, whether or not the address has an account.
Same reasoning as `INVALID_CREDENTIALS`: a different response for a known
address turns the endpoint into a list of who has registered. Your UI should say
"if that address has an account, we have sent a link" rather than confirming
anything.

The new password is subject to the same 12–128 rule as registration —
see [`VALIDATION.md`](VALIDATION.md#accounts).

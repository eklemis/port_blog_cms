<!-- GENERATED FILE - DO NOT EDIT BY HAND.
     Source: src/shared/api/error_code.rs
     Regenerate: UPDATE_DOCS=1 cargo test -p backend_actix api_errors_doc -->

# API error codes

Every error response has the same shape:

```json
{
  "success": false,
  "error": { "code": "USER_NOT_FOUND", "message": "User not found" }
}
```

`code` is the contract. **Branch on `code`, never on `message`** — messages are
prose written for humans and change without notice.

## HTTP status is not implied by the code

The *typical status* column below is the status a code is usually returned
with, but it is not a guarantee: three codes are deliberately emitted with two
statuses, because the same condition means different things at different
endpoints.

| Code | Also returned as | Where, and why |
| --- | --- | --- |
| `TOKEN_INVALID` | 400 | Email verification, where the token is a malformed path segment rather than a credential. 401 on token refresh. |
| `TOKEN_EXPIRED` | 400 | Same reason as above. |
| `INVALID_TOKEN_TYPE` | 400 | The refresh endpoint, where a non-refresh token is bad input. 401 from the auth extractor, where an access token was required. |

Treat the status as the transport-level answer and the code as the specific
one.

## Authentication and tokens

| Code | Typical status | Meaning |
| --- | --- | --- |
| `MISSING_AUTH_HEADER` | 401 Unauthorized | No Authorization header, or it was not a well-formed Bearer token. |
| `INVALID_CREDENTIALS` | 401 Unauthorized | Email and password did not match an account. Deliberately does not say which was wrong. |
| `INVALID_TOKEN` | 401 Unauthorized | The token could not be verified, or has expired. |
| `TOKEN_INVALID` | 401 Unauthorized | The token's signature or structure is not valid. 401 when it was offered as a credential, 400 when it arrived as a URL segment. |
| `TOKEN_EXPIRED` | 401 Unauthorized | The token's expiry has passed. 401 on refresh, 400 on email verification. |
| `TOKEN_NOT_YET_VALID` | 400 Bad Request | The token's not-before claim is in the future. Usually a clock-skew problem. |
| `INVALID_TOKEN_TYPE` | 401 Unauthorized | A token of the wrong kind was supplied — an access token where a refresh token was required, or the reverse. |
| `INVALID_RESET_TOKEN` | 401 Unauthorized | The password-reset token is invalid or has expired. |
| `EMAIL_NOT_VERIFIED` | 403 Forbidden | The account exists but its email address has not been confirmed. |
| `USER_DELETED` | 403 Forbidden | The account has been soft-deleted and cannot be used. |

## Authorisation

| Code | Typical status | Meaning |
| --- | --- | --- |
| `FORBIDDEN` | 403 Forbidden | The caller is authenticated but does not own the resource. |
| `USER_UNAUTHORIZED` | 401 Unauthorized | The caller may not perform this action on this account. |
| `CV_UNAUTHORIZED` | 403 Forbidden | The caller does not own this CV. |
| `POST_UNAUTHORIZED` | 403 Forbidden | The caller does not own this blog post. |

## Not found

| Code | Typical status | Meaning |
| --- | --- | --- |
| `USER_NOT_FOUND` | 404 Not Found | No user matches the given id or username. |
| `CV_NOT_FOUND` | 404 Not Found | No CV matches the given id, or it is not visible to the caller. |
| `POST_NOT_FOUND` | 404 Not Found | No blog post matches the given id or slug. |
| `PROJECT_NOT_FOUND` | 404 Not Found | No project matches the given id or slug. |
| `TOPIC_NOT_FOUND` | 404 Not Found | No topic matches the given id. |
| `MEDIA_NOT_FOUND` | 404 Not Found | No media item matches the given id. |
| `JOB_NOT_FOUND` | 404 Not Found | No job posting matched the id, or it belongs to another user. |
| `APPLICATION_NOT_FOUND` | 404 Not Found | No application matched the id, or it belongs to another user. |
| `VARIANT_NOT_FOUND` | 404 Not Found | The media item exists but not in the requested size. |
| `TARGET_NOT_FOUND` | 400 Bad Request | The attachment target named in the request does not exist. |

## Conflict

| Code | Typical status | Meaning |
| --- | --- | --- |
| `USER_ALREADY_EXISTS` | 409 Conflict | That email or username is already registered. |
| `TOPIC_ALREADY_EXISTS` | 409 Conflict | The caller already owns a topic with that title. |
| `SLUG_ALREADY_EXISTS` | 409 Conflict | Another post or project already uses that slug. |

## Request validation

| Code | Typical status | Meaning |
| --- | --- | --- |
| `VALIDATION_ERROR` | 400 Bad Request | The request body could not be deserialised. The message carries the parser's detail. |
| `INVALID_REQUEST` | 400 Bad Request | The request is structurally valid but not a combination the endpoint accepts. |
| `MISSING_FIELD` | 400 Bad Request | A required field was absent. |
| `BULK_TOO_LARGE` | 400 Bad Request | A bulk request carried more ids than the endpoint accepts in one call. |
| `BULK_EMPTY` | 400 Bad Request | A bulk request carried no ids. Guard the control rather than calling with an empty selection. |
| `SNAPSHOT_REQUIRED` | 400 Bad Request | An application cannot leave draft without a CV snapshot. Send cv_id so one can be taken. |
| `INVALID_EMAIL` | 400 Bad Request | The email address is not well formed. |
| `INVALID_PASSWORD` | 400 Bad Request | The password does not meet the strength policy. |
| `INVALID_USERNAME` | 400 Bad Request | The username contains disallowed characters or is the wrong length. |
| `INVALID_FULL_NAME` | 400 Bad Request | The full name is empty or too long. |
| `INVALID_SLUG` | 400 Bad Request | The slug is empty, too long, or contains characters outside [a-z0-9-]. |
| `INVALID_TITLE` | 400 Bad Request | The title is not acceptable for this resource. |
| `EMPTY_TITLE` | 400 Bad Request | The title was empty once trimmed. |
| `TITLE_TOO_LONG` | 400 Bad Request | The title exceeds the maximum length. |
| `INVALID_CONTENT` | 400 Bad Request | The post body is empty or otherwise unacceptable. |

## Uploads

| Code | Typical status | Meaning |
| --- | --- | --- |
| `INVALID_FILE_NAME` | 400 Bad Request | The file name is empty, too long, or contains path separators. |
| `INVALID_EXTENSION` | 400 Bad Request | The file extension is not one the upload policy allows. |
| `INVALID_MIME_TYPE` | 400 Bad Request | The declared MIME type is not one the upload policy allows. |
| `MIME_EXTENSION_MISMATCH` | 400 Bad Request | The declared MIME type and the file extension disagree. |
| `FILE_TOO_LARGE` | 400 Bad Request | The declared file size exceeds the upload limit. |
| `INVALID_DIMENSIONS` | 400 Bad Request | The declared image dimensions are missing, zero, or above the limit. |

## Media processing state

| Code | Typical status | Meaning |
| --- | --- | --- |
| `MEDIA_PENDING` | 409 Conflict | The upload has been registered but the file has not arrived yet. |
| `MEDIA_PROCESSING` | 409 Conflict | The file arrived but its variants are still being generated. Retry shortly. |
| `MEDIA_FAILED` | 409 Conflict | Variant generation failed for this media item; it will not become available. |

## Throttling and server faults

| Code | Typical status | Meaning |
| --- | --- | --- |
| `STORAGE_ERROR` | 502 Bad Gateway | The object store could not be reached or refused the operation. Upstream fault, not the caller's. |
| `RATE_LIMITED` | 429 Too Many Requests | The caller exceeded the endpoint's rate limit. The response carries Retry-After. |
| `INTERNAL_ERROR` | 500 Internal Server Error | An unexpected server-side failure. The message is deliberately generic; details are logged, not returned. |

---

54 codes in total. This file is generated from `src/shared/api/error_code.rs`; add a variant there and regenerate.

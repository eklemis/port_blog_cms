# Bucket configuration

## Why the upload bucket needs a CORS policy

Uploads do not pass through this API. `POST /api/media/upload-url` returns a
signed URL and the browser `PUT`s the bytes straight to Google Cloud Storage —
which means the browser is making a **cross-origin** request to a host this
service does not control.

A `PUT` carrying a `Content-Type` is not a simple request, so the browser sends
an `OPTIONS` preflight first and **refuses to send the `PUT` at all** unless the
bucket answers it with matching CORS headers.

Without the policy the failure is silent and looks like a broken product rather
than a misconfiguration:

- `POST /api/media/upload-url` succeeds and a `media` row is created as `pending`
- the browser blocks the transfer before it starts
- **no request ever reaches this API**, so nothing is logged and no status changes
- the row stays `pending` for ever and the upload card never moves

## Applying it

```bash
gcloud storage buckets update gs://blogport-cms-upload --cors-file=infra/gcs-cors.json
```

Check what is currently set:

```bash
gcloud storage buckets describe gs://blogport-cms-upload --format='value(cors_config)'
```

## Keeping it correct

`origin` must list every origin the console is served from. The development
entries are here; **add the production origin before the first production
upload** — the policy in this file is not applied automatically by any deploy
step, and nothing in the build will fail if it is missing or stale.

`method` needs `PUT` for the upload and `OPTIONS` for the preflight itself.
`responseHeader` needs `Content-Type`, because that is the header whose presence
makes the request preflighted in the first place.

The client sends no credentials and no custom headers, so nothing else is
required.

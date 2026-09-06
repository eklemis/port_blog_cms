# Pagination, filtering and sorting

## Which endpoints paginate

**Five, and only five.** This is the first thing to know, because the rest of
the list endpoints return everything in one response:

| Endpoint | Paginated |
| --- | --- |
| `GET /api/blog` | yes |
| `GET /api/projects` | yes |
| `GET /api/cvs` | yes |
| `GET /api/public/blog/{username}` | yes |
| `GET /api/public/projects/{username}` | yes |
| `GET /api/media/by-target/{target}` | **no** — returns all for that target |
| `GET /api/topics` | **no** — returns all |
| `GET /api/jobs` | **no** — returns all |
| `GET /api/applications` | **no** — returns all |

The unpaginated ones return a plain array in `data`, not a page object — this is
confirmed in [`openapi.json`](openapi.json), where their `data` is `type: array`
rather than a page schema. They are bounded in practice by being per-account,
but nothing caps them, so a media library or job list that grows will return
large responses. **Do not write a paging component against those four**
expecting `page` to appear later without warning.

Note there is no `GET /api/media` returning everything — media is listed per
attachment target.

## The page shape

The five paginated endpoints all answer with the same four fields:

```json
{
  "success": true,
  "data": {
    "items": [ … ],
    "page": 1,
    "per_page": 10,
    "total": 42
  }
}
```

`total` is the number of rows matching the filter **across all pages**, not the
length of `items`. Page count is `ceil(total / per_page)`.

## Query parameters

| Parameter | Default | Notes |
| --- | --- | --- |
| `page` | `1` | 1-based. `0` is treated as `1`. |
| `per_page` | `10` | `0` is treated as `10`. |

Both are plain integers, and both are optional.

### `per_page` is only capped on `/api/blog`

Blog clamps it to 100. **Projects and CVs do not** — `?per_page=100000` on those
two will attempt to return that many rows.

That is an inconsistency on our side rather than a feature, and it is worth
knowing about twice: do not rely on a server-side cap protecting your UI, and do
not send a large `per_page` on projects or CVs expecting to be trimmed.

## Filters

| Endpoint | `search` | `topic_id` | `published` |
| --- | --- | --- | --- |
| `GET /api/blog` | yes | yes | yes |
| `GET /api/projects` | yes | yes | — |
| `GET /api/cvs` | yes | — | — |

`search` is full-text over the record's own text. `topic_id` takes one UUID.
`published` takes `true` or `false`; omitting it returns both.

All filters combine with AND, and all are applied before pagination — so `total`
reflects the filtered set.

## Sorting

**The accepted values are not spelled consistently across the three endpoints.**
This is the single most likely thing on this page to cost you an afternoon:

| Endpoint | `sort` values |
| --- | --- |
| `GET /api/blog` | `Newest`, `Oldest`, `RecentlyPublished`, `RecentlyUpdated` |
| `GET /api/projects` | `Newest`, `Oldest`, `UpdatedNewest`, `UpdatedOldest` |
| `GET /api/cvs` | `newest`, `oldest`, `updatednewest`, `updatedoldest` |

Blog and projects take **PascalCase**. CVs take **lowercase**. So `?sort=Newest`
works on blog and projects and fails on CVs, where you need `?sort=newest`.

Note also that "most recently updated" is `RecentlyUpdated` on blog and
`UpdatedNewest` on projects — same idea, different word.

Omitting `sort` gives each endpoint's default, which is newest-first everywhere.

These values are generated into [`openapi.json`](openapi.json) as enums, so a
generated client will have them right. Hand-written calls are where this bites.

## Bulk is not paginated, and not atomic

`POST /api/blog/bulk`, `/api/projects/bulk`, `/api/media/bulk` take up to 100 ids
and apply one operation to each.

```json
{
  "success": true,
  "succeeded": ["…", "…"],
  "failed": [ { "id": "…", "code": "POST_NOT_FOUND" } ]
}
```

**`success: true` means the batch ran, not that every item succeeded.** Read
`failed` — each entry carries the same error code the single-item route would
have returned for that id.

This is deliberate and will not change: archiving 99 posts should not fail
because the 100th was already deleted. The reasoning is recorded in
[ADR 0011](adr/0011-writes-are-atomic-by-default.md).

Ops: `archive`, `restore`, `hard_delete`, `unpublish`, `attach_topic`,
`detach_topic`. There is no bulk `publish` — publishing is per-post considered
work, and "publish these" is ambiguous about now versus each post's scheduled
time.

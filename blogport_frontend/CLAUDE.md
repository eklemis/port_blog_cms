# blogport_frontend — working notes

SvelteKit 2 · **Svelte 5 (runes)** · Tailwind 4 · Feature-Sliced Design · TDD.

The design for this app is **already finished and specified**. Your job is to
implement it, not to redesign it. Everything below is enforced by lint, tests or
a hook — where a rule is only written down and not machined, it says so.

## Read before you write

**Do not read all of these.** Read the one that answers the question in front of
you — context is the scarce resource, not information.

### Design — what to build

Copies live in [`docs/`](docs/) because the originals are published elsewhere;
they are snapshots, and the date at the foot of each says when.

| Question you have                                                          | Read                                                                                 |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| What are the endpoints, tokens, validation rules? What must I _not_ build? | [`docs/01-frontend-handoff.html`](docs/01-frontend-handoff.html)                     |
| What screens exist, what route, what journey, what error copy?             | [`docs/02-console-blueprint.html`](docs/02-console-blueprint.html)                   |
| What's in this form? What does this component do when touched?             | [`docs/03-forms-and-interaction-spec.html`](docs/03-forms-and-interaction-spec.html) |
| What colour/contrast/focus/aria does this need?                            | [`docs/04-accessibility-spec.html`](docs/04-accessibility-spec.html)                 |
| Where does this screen link to? What do I export from Figma?               | [`docs/05-prototype-map-and-assets.html`](docs/05-prototype-map-and-assets.html)     |

Figma: `figma.com/design/zz666KBh6l5LPxphfjFP6r` — 210 screens, three widths,
light and dark, 376 prototype links across six flows.

### Backend — what the API actually does

These are **not copied here.** They live in the backend's own directory in this
same repo, they are maintained by the backend team, and they change when the API
changes. Read them where they are, so you always get the current version:

| Question you have                                     | Read                                                                                 |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Token lifetimes, rotation, what logout actually kills | [`../backend_actix/docs/AUTHENTICATION.md`](../backend_actix/docs/AUTHENTICATION.md) |
| Field rules, limits, normalisation                    | [`../backend_actix/docs/VALIDATION.md`](../backend_actix/docs/VALIDATION.md)         |
| Page params, cursors, what the envelope looks like    | [`../backend_actix/docs/PAGINATION.md`](../backend_actix/docs/PAGINATION.md)         |
| What an error code means and how to recover           | [`../backend_actix/docs/API_ERRORS.md`](../backend_actix/docs/API_ERRORS.md)         |
| Why a constraint exists that looks arbitrary          | [`../backend_actix/docs/adr/`](../backend_actix/docs/adr/) — see the two below       |

Two ADRs bind this frontend directly:
[`0009-reflections-never-feed-generation`](../backend_actix/docs/adr/0009-reflections-never-feed-generation.md)
— a reflection must never reach a prompt that produces user-facing text; and
[`0006-public-media-urls`](../backend_actix/docs/adr/0006-public-media-urls.md)
— public bucket objects cannot be un-shared once shared.

> **Never read `openapi.json`.** It is 16,000 lines and it is a generator input,
> not a document. Run `bun run gen:api` and read the generated types in
> `src/shared/api/v1.ts` instead. Reading the spec directly costs you most of
> your context and tells you less than the types do.

**Precedence when sources disagree:** the user's instruction → the backend docs
→ the Figma file → the design documents → this file → your judgement. The
backend outranks the design on what the API _does_, because the design describes
intent and the backend is what will actually answer. The Figma file wins on
anything visual; the design documents win on behaviour and copy.

**When the conflict is real, stop.** The order above resolves the ordinary
case — one source is simply more authoritative about that kind of question.
It does not license you to paper over a genuine contradiction. If a document
requires a field the API does not return, or the design shows a control the
backend cannot back, **say so and stop.** Do not invent the field, stub the
endpoint, or quietly design around it.

Every gap found this way so far has been a real defect worth someone's
attention — the cookie durations, the stale generated types, an unfinished
`/health` route that never compiled. A conflict you silently resolve is a bug
you have hidden; a conflict you report is a bug someone can fix.

## The four non-negotiables

### 1 · Svelte 5 runes. Never Svelte 4 syntax.

`svelte.config.js` sets `compilerOptions.runes: true`, so this is not a
convention you can drift from — **Svelte 4 syntax is a compile error.**
`$state` `$derived` `$effect` `$props` `$bindable`. **Not** `export let`, **not**
`$:`, **not** `svelte/store` in components.

```svelte
<script lang="ts">
	let { post, onsave }: { post: Post; onsave: (p: Post) => void } = $props();
	let dirty = $state(false);
	let canPublish = $derived(post.title.length > 0 && post.slug.length > 0);
</script>
```

Shared reactive state goes in a `.svelte.ts` module using `$state`, not a store.

### 2 · Tailwind 4, through the tokens.

Tailwind 4 is **CSS-first** — there is no `tailwind.config.js` and you must not
create one. The live theme is [`src/app/app.css`](src/app/app.css): tokens on
`:root` / `.dark`, mapped into Tailwind's namespace by `@theme inline`. That
file is the **only** place a hex literal belongs.

- Use `bg-arch-surface`, `text-arch-muted`, `border-arch-line-control`.
- **Never a raw hex, never `text-[#8f5000]`, never an arbitrary colour value.**
- `--arch-line` is a decorative hairline. The border of an _input or button_ is
  `--arch-line-control`. They are not interchangeable; see §02 of the a11y spec.

### 3 · Feature-Sliced Design.

Layers, highest to lowest. **A layer may import only from layers below it.**

```
app        → routes, providers, global styles   (src/app — note: routes live here)
pages      → one route's composition
widgets    → self-contained blocks of UI
features   → a user action that changes state
entities   → a business object and its display
shared     → ui kit, api client, config, lib    (imports from nobody)
```

Two rules that are violated most often:

- **Slices in the same layer may not import each other.** `entities/blog` may
  not import `entities/project`. If they need to share, it belongs in `shared`.
- **Import through the slice's public API only.** `$lib/entities/blog` — never
  `$lib/entities/blog/ui/blog-card.svelte`. Every slice needs an `index.ts` that
  re-exports its surface.

This repo moves SvelteKit's routes into the app layer via `svelte.config.js`
(`files.routes = 'src/app/routes'`, `files.lib = 'src'`).

**Use `$lib/…`.** Because `files.lib = 'src'`, `$lib/shared/ui` resolves to
`src/shared/ui`. An `@/*` alias is also configured and resolves to the same
place — having two spellings is how a violation gets written with whichever one
isn't being checked, so the guard checks both and you should write `$lib/`.

### 4 · TDD. Test first, always.

Vitest + `vitest-browser-svelte`. The loop is not optional:

1. **Red** — write the failing test. Run it. _Watch it fail._ A test that has
   never failed proves nothing.
2. **Green** — the least code that passes.
3. **Refactor** — with the test still green.

**The filename picks the runner** (`vite.config.ts` splits them into two
projects), and getting it wrong is the most common way a test silently never
runs the way you meant:

| File                    | Runs in                 | For                        |
| ----------------------- | ----------------------- | -------------------------- |
| `button.svelte.spec.ts` | a real Chromium browser | components                 |
| `auth.api.spec.ts`      | node                    | plain modules, server code |

Co-locate the spec beside what it tests. `expect.requireAssertions` is on, so a
test with no assertion fails rather than passing hollowly.

Test behaviour a user can observe — rendered text, roles, what a click changes,
what a screen reader is told. Not internal state, not implementation details.

**Every component spec asserts accessibility.** `expectNoA11yViolations()` from
[`$lib/shared/test/a11y`](src/shared/test/a11y.ts) runs axe against the real
rendered DOM at WCAG 2.2 AA. It catches about a third of what the Accessibility
Spec requires — the mechanical third. Focus order, live-region timing and copy
quality are still yours to check.

**Copy [`src/shared/ui/button/`](src/shared/ui/button/).** It is the reference
slice: runes props, colour only through tokens, an `index.ts` public API, and a
spec that asserts observable behaviour. Its five tests are the shape to follow.

## Commands

**This project uses [Bun](https://bun.sh), not npm.** `bun.lockb` is the
lockfile; do not create a `package-lock.json`.

```bash
bun install            # never `npm install` — it writes the wrong lockfile
bun run dev            # vite dev
bun run test           # vitest --run  (NOT `bun test` — see below)
bun run test:unit      # vitest watch — use this while red-green-refactoring
bun run check          # svelte-check (types + a11y)
bun run lint           # prettier --check && eslint
bun run guard          # the house-rules sweep, same checks as the write hook
bun run gen:api        # regenerate v1.ts from ../backend_actix/docs/openapi.json
bun run types:check    # fail if v1.ts has drifted from that spec
```

Before you say a task is done:

```bash
bun run types:check && bun run test && bun run check && bun run lint
```

> **`bun test` is not `bun run test`.** `bun test` is Bun's own built-in test
> runner. It does not understand this project's Vitest browser mode and fails
> with `vitest/browser can be imported only inside the Browser Mode` — which
> looks like a broken test and is not. Always `bun run test`.

## Things that will bite

- **Bun is the package manager, and the lockfile is how the platform knows.**
  Vercel and Cloud Build pick their package manager from whichever lockfile they
  find, so a stray `package-lock.json` silently flips the build to npm. If you
  ever run `npm install` here by reflex, delete the lockfile it wrote.
- **Media read URLs are signed and expire.** An image URL can never be written
  into stored Markdown. Post bodies hold `![alt](media:8f1b2c3d)` and resolve at
  render — signed URL in the console, public path on public pages.
- **Login succeeds for unverified accounts**, then every authoring route 403s
  with `EMAIL_NOT_VERIFIED`. Verification, not login, is the gate. Never render
  an authoring UI to an unverified user.
- **Refresh rotates.** Every `/api/auth/refresh` returns a _new_ refresh token.
  Persist both or the session silently pins to the original 7-day window.
- **Password validation is length only** (12–128). The OpenAPI example implies
  complexity rules. It is wrong. Do not add client-side rules the server lacks.
- **`svelte.config.js` moves routes into the app layer** via the `files.*`
  options, which SvelteKit now prints a deprecation warning for on every command.
  The warnings are expected and harmless today; when those options are removed,
  the FSD layout needs a different mechanism. Do not "fix" the warning by moving
  routes back to `src/routes`.
- **`src/shared/api/v1.ts` is generated — never hand-patch it.** It had silently
  rotted to 1,706 lines against a 12,531-line spec, missing every `/api/ai/*`
  and `/api/applications/*` route, so the whole Career Studio surface was
  invisible to the type checker. `bun run types:check` now fails when that drifts
  again; `bun run gen:api` fixes it. If a type you need is missing, regenerate
  before concluding the endpoint does not exist.

## Definition of done

The gate proves most of it. **Run it — do not assert it:**

```bash
bun run types:check && bun run test && bun run check && bun run lint
```

That already covers what a machine can decide: the types match the backend spec,
tests pass, runes are used, no FSD boundary is crossed, no raw hex, no missing
component spec, and axe finds no WCAG 2.2 AA violation in what you rendered.
Pasting the output beats claiming compliance — a self-assessment that says "yes,
I followed the rules" is not evidence, it is the same sentence whether or not
you did.

**Then check the five things no tool can decide.** These are the whole reason
this list exists:

- [ ] **It matches the frame.** Open the Figma screen, not just the spec.
      Spacing, hierarchy and copy, in light _and_ dark.
- [ ] **It works at all three widths** — 390, 834, 1180. No width answers with
      "please use a larger screen".
- [ ] **No invented API field.** A field that type-checks can still be wrong:
      the types say a shape exists, not that this endpoint returns it populated.
- [ ] **Error copy is the spec's copy**, not a paraphrase. The wording in the
      Console Blueprint was chosen; "Something went wrong" was not.
- [ ] **The things axe cannot see**: focus order is sensible, a live region
      announces at the right moment and not on every keystroke, motion respects
      `prefers-reduced-motion`. Accessibility Spec §09 and §15.

If a box will not tick, say which one and why. An unfinished task reported
honestly is worth more than a finished-looking one that is not.

## Git, PRs and CI

**Commit subjects are imperative sentences. No `feat:` / `fix:` prefixes.** This
repo has a house style and it is not Conventional Commits — say what the change
does, in a sentence, capitalised:

```
Name every sort value the same way
Check that wire enums are snake_case, and fix the one this found
Record that writes are atomic by default
Correct the rule migrations are held to
```

**Branches do take a prefix**: `feat/`, `fix/`, `docs/`, `chore/` — for example
`fix/sort-value-casing`. Prefix on the branch, prose on the commit.

Work reaches `main` through a pull request. Do not commit to `main` directly,
and do not force-push a branch someone else may have pulled.

**Commit or push only when you are asked to.** Finishing a task means the work
is done and the gate is green, not that it is pushed.

### CI runs the same gate you do

[`.github/workflows/frontend.yml`](../.github/workflows/frontend.yml) runs
`types:check → guard → check → lint → test` on every PR touching this app, with
bun and a real Chromium. It is the same sequence as the local gate on purpose:
if it passes here it passes there, and a green local run is a real signal rather
than a different one.

It also triggers on `backend_actix/docs/openapi.json`, because this app's types
are generated from that file — a backend-only change can break this build. That
path was missing, which is how `v1.ts` fell 11k lines behind without any CI run
noticing.

### Never commit

- **`.env`** — it is gitignored; keep it that way and never paste its contents
  into a file that is not.
- **Secrets of any kind**, including a token pasted into a test fixture.
- **`node_modules/`, `.svelte-kit/`, build output.**
- **A red gate.** If something fails and you cannot fix it, say so in the PR
  rather than committing around it.

`src/shared/api/v1.ts` _is_ committed, deliberately — it is generated, but
committing it means the build needs no backend, and `types:check` is what keeps
it honest. Regenerate it with `bun run gen:api`; never hand-edit it.

## Do not build these

No endpoint exists. Full list and reasoning in §04 of the Frontend Handoff.

- Change email or password while signed in
- Sign out of all devices
- A combined "all media" library view
- Reactivating a closed account

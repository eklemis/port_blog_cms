-- Demo data for the test database.
--
-- Refuses to run unless it is connected to the database it expects. `eksanto`
-- exists in production too, and this script deletes before it inserts, so a
-- stale DATABASE_URL in the shell would otherwise be enough to delete
-- production rows. Override deliberately when your test database is named
-- something else:
--
--   psql "$DATABASE_URL" -f scripts/seed_test_db.sql                  # expects cms
--   psql "$DATABASE_URL" -v seed_db=mytestdb -f scripts/seed_test_db.sql
--
-- Repeatable: every row it writes has a fixed id prefix, and it deletes those
-- rows before inserting, so running it twice leaves the same database rather
-- than two copies. It touches nothing else — rows created by hand, or by using
-- the app, survive it.
--
--   11111111- posts      22222222- topics    33333333- jobs
--   44444444- CVs        55555555- snapshots 66666666- applications
--
-- Owner is whichever account is named below; nothing here creates accounts or
-- touches credentials.

\set ON_ERROR_STOP on

-- Default the expected database name unless one was passed with -v seed_db=…
\if :{?seed_db}
\else
\set seed_db cms
\endif

BEGIN;

-- The guard. Runs before anything is deleted.
--
-- The expected name goes through set_config rather than straight into the
-- block below: psql does not substitute its variables inside a $$-quoted
-- body, so `:'seed_db'` there is a syntax error rather than a check.
SELECT set_config('seed.expected_database', :'seed_db', true);

DO $$
BEGIN
    IF current_database() <> current_setting('seed.expected_database') THEN
        RAISE EXCEPTION
            'refusing to seed: connected to "%", expected "%"',
            current_database(), current_setting('seed.expected_database')
            USING HINT = 'check DATABASE_URL, or pass -v seed_db=<name> if this really is your test database';
    END IF;
END $$;

-- The owner. Chosen by username so the script does not carry an id that only
-- matches one database.
CREATE TEMP TABLE seed_owner AS
SELECT id FROM users WHERE username = 'eksanto';

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM seed_owner) THEN
        RAISE EXCEPTION 'no user named eksanto in this database';
    END IF;
END $$;

-- ── Clear previous runs ─────────────────────────────────────────────────
DELETE FROM applications    WHERE id::text LIKE '66666666-%';
DELETE FROM cv_snapshots    WHERE id::text LIKE '55555555-%';
DELETE FROM resumes         WHERE id::text LIKE '44444444-%';
DELETE FROM jobs            WHERE id::text LIKE '33333333-%';
DELETE FROM blog_post_topics WHERE blog_post_id::text LIKE '11111111-%'
                                OR topic_id::text LIKE '22222222-%';
DELETE FROM topics          WHERE id::text LIKE '22222222-%';
DELETE FROM blog_posts      WHERE id::text LIKE '11111111-%';

-- ── Topics ──────────────────────────────────────────────────────────────
INSERT INTO topics (id, user_id, title, description, is_deleted, created_at, updated_at)
SELECT ('22222222-0000-4000-8000-00000000000' || n)::uuid, o.id, t, d, false,
       now() - make_interval(days => 120), now() - make_interval(days => 120)
  FROM seed_owner o,
       (VALUES ('1', 'Rust',      'Posts about the Rust language'),
               ('2', 'Svelte',    'Front-end notes, mostly SvelteKit'),
               ('3', 'Postgres',  'Schema design and the things it catches'),
               ('4', 'Career',    'Applications, CVs and the search itself')
       ) AS v(n, t, d);

-- ── Posts ───────────────────────────────────────────────────────────────
-- Five live, three drafts, two scheduled, four archived: enough to fill a
-- page of ten and leave a second page, and to give every state something.
INSERT INTO blog_posts (id, user_id, title, slug, excerpt, content, published_at, is_deleted, created_at, updated_at)
SELECT ('11111111-0000-4000-8000-0000000000' || n)::uuid, o.id, title, slug, excerpt, body,
       CASE WHEN pub_days IS NULL THEN NULL ELSE now() - make_interval(days => pub_days) END,
       false,
       now() - make_interval(days => created_days),
       now() - make_interval(days => created_days)
  FROM seed_owner o,
       (VALUES
         ('01', 'Hexagonal layout, and what it costs', 'hexagonal-layout-and-what-it-costs',
          'Ports and adapters, minus the diagram worship', E'The shape earns its keep at the edges.\n\nEverything the database knows stays behind a port.', 40, 45),
         ('02', 'A slug is a promise', 'a-slug-is-a-promise',
          'Why renaming one is a migration, not an edit', E'A published slug is someone else’s bookmark.', 32, 36),
         ('03', 'Reading a query plan without fear', 'reading-a-query-plan-without-fear',
          'EXPLAIN, one line at a time', E'Start at the innermost node and read outwards.', 21, 25),
         ('04', 'The N+1 you cannot see', 'the-n-plus-one-you-cannot-see',
          'It looks like one request until the list grows', E'One query per row hides well at five rows.', 12, 15),
         ('05', 'Dates that lie quietly', 'dates-that-lie-quietly',
          'On inferring a timestamp you never recorded', E'A column that is wrong without saying so is worse than a blank one.', 4, 6),
         ('06', 'Draft: the error vocabulary', 'draft-the-error-vocabulary',
          'Ten classes, and who decides the wording', E'Still deciding where the message belongs.', NULL, 9),
         ('07', 'Draft: shaping the tracker', 'draft-shaping-the-tracker',
          NULL, E'Notes towards the applications screen.', NULL, 5),
         ('08', 'Draft: untitled thoughts on caching', 'draft-untitled-thoughts-on-caching',
          NULL, '', NULL, 2),
         ('09', 'Scheduled: migrations before deploys', 'scheduled-migrations-before-deploys',
          'Why the old build has to still work', E'Migrations land while yesterday’s code is serving.', -3, 8),
         ('10', 'Scheduled: what a snapshot is for', 'scheduled-what-a-snapshot-is-for',
          'Freezing a CV so history stops moving', E'The document is stored, not referenced, and that is the point.', -10, 7),
         ('11', 'Old notes on serde', 'old-notes-on-serde',
          'Superseded by the derive docs', E'Left here for the archive screen.', 90, 95),
         ('12', 'An abandoned benchmark', 'an-abandoned-benchmark',
          NULL, E'Numbers I no longer trust.', 80, 88),
         ('13', 'Duplicate of the slug post', 'duplicate-of-the-slug-post',
          NULL, E'Written twice by accident.', 70, 75),
         ('14', 'A post about nothing', 'a-post-about-nothing',
          NULL, '', NULL, 60)
       ) AS v(n, title, slug, excerpt, body, pub_days, created_days);

-- Archive the last four through the is_deleted transition, so the trigger
-- stamps deleted_at exactly as it will in real use.
UPDATE blog_posts SET is_deleted = true
 WHERE id::text IN ('11111111-0000-4000-8000-000000000011',
                    '11111111-0000-4000-8000-000000000012',
                    '11111111-0000-4000-8000-000000000013',
                    '11111111-0000-4000-8000-000000000014');

-- Spread the archive dates out. Touching deleted_at alone does not fire the
-- is_deleted trigger, so these stick.
UPDATE blog_posts SET deleted_at = now() - make_interval(days => 30) WHERE id = '11111111-0000-4000-8000-000000000011'::uuid;
UPDATE blog_posts SET deleted_at = now() - make_interval(days => 18) WHERE id = '11111111-0000-4000-8000-000000000012'::uuid;
UPDATE blog_posts SET deleted_at = now() - make_interval(days => 7)  WHERE id = '11111111-0000-4000-8000-000000000013'::uuid;
UPDATE blog_posts SET deleted_at = now() - make_interval(days => 1)  WHERE id = '11111111-0000-4000-8000-000000000014'::uuid;

-- ── Topic links ─────────────────────────────────────────────────────────
-- Some posts carry two topics, some one, some none: the Topics column needs
-- all three cases to be worth looking at.
INSERT INTO blog_post_topics (blog_post_id, topic_id, created_at)
SELECT ('11111111-0000-4000-8000-0000000000' || p)::uuid,
       ('22222222-0000-4000-8000-00000000000' || t)::uuid,
       now() - make_interval(days => 30)
  FROM (VALUES ('01','1'), ('01','3'), ('02','1'), ('03','3'), ('04','3'),
               ('04','1'), ('05','3'), ('06','1'), ('09','3'), ('10','4'),
               ('11','1'), ('13','1')
       ) AS v(p, t);

-- ── CVs, and snapshots frozen from them ─────────────────────────────────
INSERT INTO resumes (id, user_id, display_name, role, bio, photo_url,
                     core_skills, educations, experiences, highlighted_projects, contact_info,
                     created_at, updated_at, is_deleted, language)
SELECT ('44444444-0000-4000-8000-00000000000' || n)::uuid, o.id, 'Eklemis Santo', role,
       'Backend-leaning full-stack, Rust and Postgres.', '',
       '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb,
       now() - make_interval(days => 100), now() - make_interval(days => 100), false, 'en'
  FROM seed_owner o,
       (VALUES ('1', 'Backend'), ('2', 'Full-stack')) AS v(n, role);

-- Built the way CvSnapshotStorePostgres builds them, so the document has the
-- same shape the application code reads back.
INSERT INTO cv_snapshots (id, cv_id, user_id, document, created_at)
SELECT ('55555555-0000-4000-8000-00000000000' || right(r.id::text, 1))::uuid, r.id, r.user_id,
       jsonb_build_object(
         'id', r.id, 'user_id', r.user_id, 'role', r.role,
         'display_name', r.display_name, 'bio', r.bio, 'photo_url', r.photo_url,
         'core_skills', r.core_skills, 'educations', r.educations,
         'experiences', r.experiences, 'highlighted_projects', r.highlighted_projects,
         'contact_info', r.contact_info),
       now() - make_interval(days => 60)
  FROM resumes r
 WHERE r.id::text LIKE '44444444-%';

-- ── Jobs ────────────────────────────────────────────────────────────────
INSERT INTO jobs (id, user_id, title, company, location, seniority,
                  required_skills, nice_to_have, source_url, source_text,
                  is_deleted, created_at, updated_at)
SELECT ('33333333-0000-4000-8000-00000000000' || n)::uuid, o.id, title, company, location, seniority,
       skills::jsonb, nice::jsonb, url, '', false,
       now() - make_interval(days => created_days), now() - make_interval(days => created_days)
  FROM seed_owner o,
       (VALUES
         ('1', 'Backend Engineer',        'Keystone Labs',  'Remote',          'Mid',    '["Rust","Postgres"]',      '["Actix"]',        'https://example.com/jobs/1', 50),
         ('2', 'Platform Engineer',       'Northwind',      'Singapore',       'Senior', '["Rust","Kubernetes"]',    '["Terraform"]',    'https://example.com/jobs/2', 42),
         ('3', 'Full-stack Developer',    'Brightseed',     'Remote',          'Mid',    '["TypeScript","Svelte"]',  '["Rust"]',         'https://example.com/jobs/3', 30),
         ('4', 'Rust Engineer',           'Halcyon Data',   'Berlin (hybrid)', 'Senior', '["Rust","Tokio"]',         '["gRPC"]',         'https://example.com/jobs/4', 21),
         ('5', 'API Engineer',            'Fernweh',        'Remote',          'Mid',    '["Postgres","OpenAPI"]',   '["SQLx"]',         'https://example.com/jobs/5', 12),
         ('6', 'Senior Backend Engineer', 'Quiet Harbour',  'Jakarta',         'Senior', '["Rust","Distributed"]',   '["Kafka"]',        'https://example.com/jobs/6', 4)
       ) AS v(n, title, company, location, seniority, skills, nice, url, created_days);

-- ── Applications ────────────────────────────────────────────────────────
-- One draft with no snapshot (the only state the constraint allows that in),
-- the rest sent, spread across the pipeline and across both CVs.
INSERT INTO applications (id, user_id, job_id, cv_snapshot_id, status, applied_at,
                          next_action, next_action_at, is_deleted, created_at, updated_at)
SELECT ('66666666-0000-4000-8000-00000000000' || n)::uuid, o.id,
       ('33333333-0000-4000-8000-00000000000' || job)::uuid,
       CASE WHEN snap IS NULL THEN NULL
            ELSE ('55555555-0000-4000-8000-00000000000' || snap)::uuid END,
       status,
       CASE WHEN applied_days IS NULL THEN NULL
            ELSE now() - make_interval(days => applied_days) END,
       next_action,
       CASE WHEN due_days IS NULL THEN NULL
            ELSE now() + make_interval(days => due_days) END,
       false,
       now() - make_interval(days => created_days),
       now() - make_interval(days => created_days)
  FROM seed_owner o,
       (VALUES
         ('1', '1', '1',  'interview', 30, 'Prepare the system design round',  3,    32),
         ('2', '2', '1',  'screening', 20, 'Chase the recruiter',              1,    22),
         ('3', '3', '2',  'applied',   14, '',                                 NULL, 16),
         ('4', '4', '1',  'rejected',  35, '',                                 NULL, 38),
         ('5', '5', '2',  'offer',     10, 'Reply to the offer',               2,    12),
         ('6', '6', NULL, 'draft',     NULL, 'Finish tailoring the CV',        5,    3),
         ('7', '1', '2',  'no_reply',  60, '',                                 NULL, 62)
       ) AS v(n, job, snap, status, applied_days, next_action, due_days, created_days);

COMMIT;

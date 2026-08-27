# CLAUDE.md

Guidance for working in this repository.

## What this project is

`feed-rs` ("feed-me") is a Rust service that manages **threat-intelligence feeds** of
IPs, domains and URLs, and serves them to firewalls and other security appliances for
use in blocklists / allowlists.

Two faces:

1. **Feed endpoint** — serves a feed as a plain-text file, one entry per line
   (`GET /feed/{name}`). This is what the appliances poll; it must stay fast and cheap.
2. **Admin console** — an SPA for managing feeds, feed entries and users, backed by a
   JSON API under `/api`.

## Tech stack

- **Language:** Rust, edition 2024.
- **Web framework:** `actix-web` 4 (`actix-files` for static assets,
  `actix-web-httpauth` for bearer auth).
- **Database:** PostgreSQL, accessed via `sqlx` 0.7 (`runtime-tokio`, no compile-time
  query checking — `default-features = false`). Migrations run automatically at
  startup from `./migrations` via `sqlx::migrate!`.
- **Auth:** bearer tokens; passwords hashed with Argon2 (`argon2` crate).
- **Logging:** `log` + `simplelog`, wired up in `src/log.rs` (see "Logging" below).
- **Frontend:** intended to be a **simple Vue** SPA served as static files from
  `./public`. Keep it minimal — no heavy build tooling unless it becomes necessary.

## Layout

```
src/
  main.rs            actix app setup, DB pool, migrations, bearer auth middleware, bind
  error.rs           ApiError enum + From conversions
  log.rs             logging_bootstrap(): per-sink log targets, file/stderr, SIGHUP reopen
  utils.rs           Argon2 create_password / verify_password
  controller/
    feed.rs          serve_feed (text endpoint) + /api/feed CRUD handlers
  model/
    mod.rs           Window (pagination) helper
    feed.rs          Feed, FeedType, InsertFeedData; feeds table access
    entry.rs         IPEntry / URLEntry / DomainEntry via make_entry_type! macro
    user.rs          User, Group; users table access
migrations/          sqlx SQL migrations (applied on startup)
scripts/             one-off SQL (DB/user bootstrap)
public/              static frontend (Vue SPA target)
```

## Feed types are handled separately on purpose

Each feed type has its **own table** (`ip_entries`, `domain_entries`, `url_entries`)
and its own generated entry type (`IPEntry`, `DomainEntry`, `URLEntry`, produced by the
`make_entry_type!` macro in `src/model/entry.rs`). This is deliberate — it lets each
type use the right column type and indexing for performance and disk usage
(e.g. IPs stored as network/`BYTEA`, URLs as long `VARCHAR`). When adding behaviour,
prefer extending the macro over special-casing one type. Adding behaviour (a new type)
should be a last resort maneuver, and only if expressed by the user.

`serve_feed` dispatches on `feed.feed_type` to the matching entry type, streams
`value\n` lines straight from the DB, and computes/caches an MD5 digest of the body on
the `feeds` row (`digest`) to be later used by a /feed?hash endpoint that will be checked
by appliances as a manner to prevent new fetches that will consume network and
processing.

Entries carry `enabled` and `valid_until` — the feed endpoint only emits rows that are
enabled and not expired.

## Common commands

```bash
# build / run (needs a .env — see below)
cargo run
cargo build --release

cargo check
cargo clippy --all-targets
cargo fmt

cargo test
```

System deps for building (OpenSSL / pkg-config):

```bash
sudo apt install pkg-config libssl-dev
```

## Configuration

Config comes from a `.env` file (loaded via `dotenvy`; **required** — the app panics
if missing). `.env` is gitignored. Known variables:

| Variable             | Default       | Meaning                                             |
| -------------------- | ------------- | -------------------------------------------------- |
| `DATABASE_URL`       | —             | Postgres connection (read by `sqlx` / `PgConnectOptions`) |
| `DB_POOL_MAX_CONNS`  | `5`           | Max DB pool connections                             |
| `BIND_HOST`          | `127.0.0.1`   | Listen host                                         |
| `BIND_PORT`          | `8080`        | Listen port                                         |
| `APP_LEVEL`          | `stderr`      | `app` log sink target: `stderr` or a file path      |
| `ACCESS_LEVEL`       | `stderr`      | `access` log sink target: `stderr` or a file path   |
| `APP_LEVEL_LEVEL`    | `info`        | Level filter for the `app` sink                     |
| `ACCESS_LEVEL_LEVEL` | `info`        | Level filter for the `access` sink                  |

(The `*_LEVEL_LEVEL` naming is what `src/log.rs` currently constructs; revisit if you
touch that file.)

## Logging

Two independent sinks, each filtered to a log target: `feed-rs::app` (application) and
`feed-rs::access` (actix request log). Each can go to stderr or a file; file sinks
reopen on `SIGHUP` for logrotate. Emit application logs with an explicit target, e.g.:

```rust
log::warn!(target: &format!("{}::app", crate::APP_NAME), "…");
```

## Database

- Schema lives in `migrations/`; applied automatically at startup. Add a new timestamped
  migration file rather than editing existing ones once they've shipped.
- `scripts/create_user.sql` bootstraps a local DB role/database.
- The initial migration seeds an `admin` user (login `admin` / password `gofeed` per the
  comment). Treat that as dev-only.
- `feeds.type` is a Postgres `ENUM` (`FeedType`: `ip`, `domain`, `url`).

## Status / gotchas

This is an **early-stage WIP**. Expect rough edges:

- Bearer auth middleware (`bearer_validator` in `main.rs`) currently accepts any token —
  real token validation is not implemented yet. `/api/login` is unauthenticated.
- Several API handlers are `todo!()` (`update_feed`, `delete_feed`) and there is no
  user/auth controller wired under `/api` yet.
- `src/model/*` and the migration schema have drifted in places (column names, bind
  counts, `User.password_hash` field). Verify against the actual migration before
  relying on a query.
- No test suite yet.

When filling these in, keep the two-surface split clean: the feed endpoint stays a
lean streaming text response; everything management-related goes through `/api` + the
Vue console.

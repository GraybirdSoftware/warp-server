# warp-server

A self-hostable implementation of the [Binary Ninja WARP](https://docs.binary.ninja/guide/warp.html)
server. It speaks the same HTTP API as `https://warp.binary.ninja`
(mirroring its [OpenAPI document](https://warp.binary.ninja/api/openapi.json)),
so the stock WARP plugin that ships with Binary Ninja can push and pull
function signatures from it unmodified.

Single binary, SQLite storage, no external services required.

## Quick start

```sh
cp .env.example .env            # optional; defaults work for local use
cargo run -- admin create you@example.com you
#   Created admin 'you' <you@example.com> (id 1).
#   API key (shown once, store it now): warp_9f1c...
cargo run                       # serves on http://127.0.0.1:8080
```

Then, in Binary Ninja, set:

| Setting                          | Value                     |
| -------------------------------- | ------------------------- |
| `warp.container.serverUrl`       | `http://127.0.0.1:8080`   |
| `warp.container.serverApiKey`    | the key printed above     |
| `network.enableWARP`             | on                        |

Restart Binary Ninja; the log should say
`Server 'http://127.0.0.1:8080' connected, logged in as user 'you'`.
Create a source from the WARP sidebar (or `POST /api/v1/sources`), run the
processor on a binary and commit — the functions land in this server. Other
users of the same source will have them matched on fetch.

The plugin only fetches from sources tagged `official` or `trusted` by
default; either tag your sources (admins: `POST /api/v1/sources/{id}` with
`{"tags": ["trusted"]}`) or adjust `warp.fetcher.allowedSourceTags`.

## Authentication & roles

* **API keys** — `Authorization: Bearer warp_...`. Create them with
  `POST /api/v1/users/me/keys`, the CLI (`warp-server admin key <email>`),
  or as an admin for another user. Only a SHA-256 digest is stored.
* **Browser sessions** — `POST /api/v1/auth/login` starts an OAuth 2.0
  authorization-code flow with whatever provider is configured through the
  `OAUTH_*` variables (GitHub, GitLab, Google, Keycloak, Authentik, ...).
  The callback sets an `HttpOnly` cookie. Without `OAUTH_*` configured the
  login route answers `503` and API keys are the only way in.
* **Roles** — `User` and `Admin`. Read routes are public (guest access, like
  the upstream server). Writes need a key. Admin-only: creating/deleting
  users, changing roles, listing other users' keys, setting source tags.
  Members of a source (and admins) may push files to it, rename it, change
  its membership, delete it and delete its commits.

The first admin comes from `warp-server admin create`, from
`WARP_BOOTSTRAP_ADMIN_EMAIL`, or — when OAuth is configured — the first
person to log in on an empty server.

## CLI

```
warp-server [serve]                          run the HTTP server (default)
warp-server admin create <email> [username]  create an admin and print an API key
warp-server admin key <email> [key-name]     mint a new API key for an existing user
warp-server admin promote <email>            grant the Admin role to a user
```

## Configuration

All settings come from the environment (a `.env` file is read if present).

| Variable                       | Default                          | Purpose                                   |
| ------------------------------ | -------------------------------- | ----------------------------------------- |
| `DATABASE_URL`                 | `sqlite://sqlite.db?mode=rwc`    | SQLite file; created and migrated on start |
| `WARP_HOST` / `WARP_PORT`      | `127.0.0.1` / `8080`             | Bind address                              |
| `WARP_PUBLIC_URL`              | `http://$HOST:$PORT`             | Base URL used for OAuth redirects         |
| `WARP_BOOTSTRAP_ADMIN_EMAIL`   | —                                | Create this admin if no users exist       |
| `WARP_BOOTSTRAP_ADMIN_USERNAME`| local part of the e-mail         |                                           |
| `OAUTH_CLIENT_ID` / `_SECRET`  | —                                | Enables browser login when set            |
| `OAUTH_AUTH_URL` / `OAUTH_TOKEN_URL` / `OAUTH_USERINFO_URL` | — | Provider endpoints (required with client id) |
| `OAUTH_SCOPES`                 | `openid email profile`           |                                           |
| `OAUTH_REDIRECT_URL`           | `$WARP_PUBLIC_URL/api/v1/auth/o/callback` | Register this with the provider  |
| `OAUTH_EMAIL_FIELD` / `OAUTH_USERNAME_FIELD` | `email` / `preferred_username` | Fields of the user-info JSON  |
| `WARP_AUTO_REGISTER`           | `true`                           | Create accounts on first OAuth login      |
| `WARP_SESSION_TTL_HOURS`       | `720`                            | Browser session lifetime                  |
| `RUST_LOG`                     | `info,sqlx=warn`                 | Log filter                                |

## API

Base path `/api/v1`. `POST .../query` routes take a JSON body of optional
filters plus `limit`/`page` (zero-based) and answer
`{ "items": [...], "total_pages": n, "total_results": n }`.

| Area      | Routes                                                                                                                         |
| --------- | ------------------------------------------------------------------------------------------------------------------------------ |
| status    | `GET /status`                                                                                                                  |
| auth      | `POST /auth/login`, `POST /auth/logout`, `GET /auth/o/callback`                                                                |
| users     | `POST /users` (admin), `POST /users/query`, `GET /users/{id}`, `DELETE /users/{id}` (admin), `GET|POST /users/{id}/keys`, `PATCH /users/{id}/role` (admin), `PATCH /users/{id}/username`, `GET /users/me`, `GET|POST /users/me/keys` |
| sources   | `POST /sources`, `POST /sources/query`, `GET|POST|DELETE /sources/{id}`, `GET /sources/{id}/tags`, `GET /sources/{id}/users` |
| files     | `POST /files` (multipart: `file`, `name`, `source`, `description?`), `POST /files/json` (base64 `file`) → `{ "commit_id" }`  |
| commits   | `POST /commits/query`, `GET|DELETE /commits/{id}`                                                                              |
| functions | `POST /functions/query` (`"format": "json" \| "flatbuffer"`), `POST /functions/query/source`, `POST /functions/data`, `GET /functions/{id}[/comments\|/constraints\|/data\|/symbol\|/type]` |
| types     | `POST /types/query` (`format` as above), `POST /types/data`, `GET /types/{guid}`, `GET /types/{guid}/data`                    |
| symbols   | `POST /symbols/query`, `GET /symbols/{id}`                                                                                     |
| targets   | `POST /targets/query`, `GET /targets/query?platform=&arch=` (id as `text/plain`), `GET /targets/{id}`                          |
| search    | `GET /search?q=&kind=&limit=&offset=&function_guid=&commit_id=&source_id=&source_tags[]=&retrieve_data=`                        |

`flatbuffer` responses and the `/data` routes return WARP files / objects
exactly as produced by the [`warp`](https://github.com/Vector35/warp) crate,
one signature chunk per target. `POST /functions/data` and
`POST /types/data` are batch variants (`{ "ids": [...] }`) used by recent
plugin builds; they are not in the upstream OpenAPI document.

Pushing a `.warp` file creates a commit and stores every function (with its
symbol, constraints, comments and type), every type chunk entry and the
target of each chunk. Symbols, targets and types are de-duplicated; the
original file is kept alongside the commit. Deleting a commit or a source
removes everything it introduced.

## Development

```sh
cargo test                       # integration tests spin up a server per test on a temp SQLite file
GET /introspection               # actix route dump (dev aid)
```

Queries are runtime-checked (`sqlx::query_as` / `QueryBuilder`), so the crate
builds without a database present. Schema changes go in `migrations/`
(`sqlx migrate add <name>` if you have `sqlx-cli`); they are applied
automatically at startup and by the test harness.

## Storage layout

SQLite, one file. `users`, `api_keys`, `sessions`, `oauth_states`,
`source`, `source_users`, `source_tags`, `commits`, `files`, `targets`,
`symbols`, `types`, `functions`, `function_constraints`, `function_comments`.
UUIDs are stored as text, timestamps as RFC 3339 text, WARP payloads as
blobs.

## Binary Ninja compatibility notes

* The plugin sends `Content-Encoding: gzip` on every request while the body
  is usually plain JSON. The server sniffs the gzip magic bytes and accepts
  both forms (`src/middleware.rs`).
* Targets are created when a file is pushed. Until a source has been pushed
  for a given platform/architecture, `targets/query` finds nothing and the
  plugin skips fetching for that target — expected on an empty server.
* Plain `http://` works for local use; put a TLS proxy in front for anything
  reachable from other machines, since API keys travel in the header.

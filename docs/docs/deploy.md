---
sidebar_position: 6
title: Deploy (operator guide)
---

# Deploying Asgard

Asgard ships as **one statically-linked binary** that serves everything on a
single port: the web dashboard (`/`), the REST API (`/api/*`), GraphQL
(`/graphql`), and the **remote MCP server** (`/mcp`, Streamable HTTP) that agents
connect to. There is no separate frontend build, no sidecar, no message broker.

This guide gets you from nothing to a reachable, governed deployment. It branches
into two paths — pick one:

- **POC-local** — built-in local users, no external identity provider, no live
  cloud provisioning. Fully usable, MCP included. The fastest way to a real test
  deployment.
- **Enterprise** — OIDC/Auth0 single sign-on for humans **and** armed Auth0 (or
  AWS) provisioning. Layered on top of the POC path; adopt it once the basics
  work.

The recommended method is deliberate: **deploy the POC path first, knowing it
will hit edges in your environment, find where it stops, then iterate.**

---

## The auth ladder

Asgard is **secure by default and never ships wide-open**, but it does not force
an identity provider on you. Three rungs:

| Rung | What | When |
|---|---|---|
| **1 — local users** | Built-in username/password accounts + sessions. On first boot, if no admin exists and `ASGARD_ADMIN_PASSWORD` is unset, Asgard **generates an admin password and logs it once**. | Default. Zero external dependencies. |
| **2 — OIDC / SSO** | Authorization-code login against your IdP (Auth0, Okta, Entra, …). Coexists with local users, so the local admin remains a break-glass account. | Enterprise. Set the `ASGARD_OIDC_*` env. |
| **3 — dev escape hatch** | `ASGARD_DEV_INSECURE=1` disables human-session enforcement. **Off by default, only honored on a loopback bind, logs a loud warning.** | Throwaway local hacking only. Never in a deployment. |

Two things are gated independently of the human rung and are **always on**:

- **Agent inference** (`/api/gateway/chat`) is gated by a per-project virtual key.
- **The MCP server** (`/mcp`) is gated by a per-project virtual key on every
  request — even when rung 3 is enabled. A missing or invalid key is `401`.

So a human signs in (rung 1 or 2) to use the dashboard; an agent presents a
project virtual key to use `/mcp`. Different credentials, same enforcement.

---

## Prerequisites

- A host that can run the binary (or the container). **That's it — nothing else is
  required to get going.** No reverse proxy, no Redis, no Kubernetes: run the
  binary, reach it over `http://<host>:8080`, and sign in. TLS is an *optional*
  production upgrade (see below), not a prerequisite.
- **Postgres** for anything beyond a single-box trial (SQLite is the default and
  is fine for a first smoke test — no external DB needed to start).
- A 32-byte master key for the secret store (optional for a smoke test; source it
  from your KMS for a pilot).

---

## Optional: TLS via a reverse proxy

**You do not need a proxy to run Asgard.** Over plain http it serves the dashboard,
API, and MCP, and sign-in works (the session cookie is only marked `Secure` when a
request actually arrives over TLS, so plain http isn't broken by it). For a pilot
you'll still want TLS — the simplest way is to put any reverse proxy in front and
let it terminate TLS. If you do, set two headers so Asgard adapts correctly:

- **`X-Forwarded-Proto: https`** — tells Asgard the edge is TLS, so it marks the
  session cookie `Secure`. (Absent → plain http assumed → cookie not `Secure`, and
  login still works.)
- **`X-Forwarded-For`** — login brute-force throttling keys on the client IP from
  this header. Without it, all sources share one throttle bucket (still safe, just
  coarser).

Route `/`, `/api/*`, `/graphql`, and `/mcp` to the Asgard upstream. No WebSocket
upgrade is needed (MCP uses Streamable HTTP, i.e. plain POST + SSE responses), but
**don't buffer `/mcp` responses** if you want streaming to flow promptly.

Caddy makes this automatic:

```caddy
asgard.example.com {
    reverse_proxy asgard:8080
    # Caddy terminates TLS and sets X-Forwarded-Proto / X-Forwarded-For for you.
}
```

nginx:

```nginx
server {
    listen 443 ssl;
    server_name asgard.example.com;
    # ssl_certificate / ssl_certificate_key ...
    location / {
        proxy_pass http://asgard:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;       # must be https
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_buffering off;                              # let /mcp SSE stream
    }
}
```

## Step 1 — Postgres

SQLite (the default) needs nothing. For a real pilot, run Postgres and point
Asgard at it.

```bash
docker run -d --name asgard-pg \
  -e POSTGRES_PASSWORD=change-me -e POSTGRES_DB=asgard \
  -p 5432:5432 -v asgard-pg:/var/lib/postgresql/data \
  postgres:16-alpine
```

Or with compose, alongside Asgard:

```yaml
# docker-compose.yml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_PASSWORD: change-me
      POSTGRES_DB: asgard
    volumes: [ "asgard-pg:/var/lib/postgresql/data" ]
  asgard:
    build: .            # or image: your-registry/asgard:tag
    depends_on: [ db ]
    ports: [ "8080:8080" ]
    environment:
      ASGARD_DATABASE_URL: postgres://postgres:change-me@db:5432/asgard
      ASGARD_BIND: 0.0.0.0:8080
      ASGARD_SECRET_KEY: ${ASGARD_SECRET_KEY}        # 64 hex chars from your KMS
      ASGARD_ADMIN_PASSWORD: ${ASGARD_ADMIN_PASSWORD} # optional; else auto-generated + logged
    volumes:
      - ./asgard.yaml:/asgard.yaml:ro
    command: [ "serve", "--config", "/asgard.yaml" ]
volumes: { asgard-pg: {} }
```

Asgard runs its own migrations on boot against whatever `ASGARD_DATABASE_URL`
points to; the same schema works on SQLite and Postgres.

> **Run one replica.** The cost-rollup and secret-rotation background loops assume
> a single writer. Asgard does not do leader election (by design). Scale vertically
> for the pilot; one replica is correct.

## Step 2 — The master key

The built-in secret store encrypts secret values with a 32-byte master key.
Source it from your KMS and inject it as **64 hex characters**:

```bash
export ASGARD_SECRET_KEY=$(openssl rand -hex 32)   # or fetch from your KMS
```

It can also be set as `provisioning.secrets.master_key_hex` in `asgard.yaml`, but
the env var wins and is preferred so the key never lands in a config file. If
unset, a built-in dev key is used — **fine for a smoke test, not for a pilot.**

> **The master key is load-bearing and must stay stable.** Secret values are
> encrypted with it; there is no re-encrypt-on-rotate. If the key changes, every
> stored secret becomes undecryptable. Keep it in your KMS (not in the DB), and
> **back up the database** — the DB holds the encrypted secrets, the KMS holds the
> key, and you need both. Rotating the key is a deliberate migration, not a config
> tweak. If you arm provisioning, the terraform state under `work_dir` (in `/data`)
> must also be on **durable** storage — lose it and you orphan real cloud resources.

## Step 3 — `asgard.yaml`

Provisioning, the group/cost-center allowlist, and catalog sources come from a
small config file mounted at a path you pass with `--config`. A minimal POC file:

```yaml
# Cost-centers a project may register against. Empty = open mode (any group).
groups:
  - { key: platform, display_name: Platform, cost_center: CC-100 }
  - { key: research, display_name: Research, cost_center: CC-200 }

# Which registration fields are mandatory. Defaults (shown) keep the strict
# posture; relax them so a solo founder/CEO can self-register without inventing a
# separate manager or a cost-center group.
registration:
  require_manager: true   # false → manager optional, defaults to the owner (self-manage)
  require_group: true     # false → group optional (ungrouped, blank cost-center)
  require_cost_center: false  # reserved; cost-center derives from group today
```

The governance operating model ships with the policy-doc defaults baked in, so
none of the following is required to boot. Add a block only to override a default:

```yaml
# Per-tier evidence required to promote into a tier (keys: light-operational /
# wide-operational / critical-path). Any tier you list replaces that tier's
# shipped default; tiers you omit keep theirs.
classification_requirements:
  wide-operational: [repo_or_source_url, support_contact, runbook_url, monitoring_or_logs_url]

# Lifecycle review-date engine. Defaults shown.
review:
  poc_window_days: 90    # first review deadline for a new POC
  auto_extensions: 1     # automatic +window grants before a human must decide
  sweep_secs: 86400      # how often the background sweep flags overdue reviews

# Portfolio-metric thresholds (the Governance dashboard tab / governance_metrics).
governance:
  maintainer_min: 2      # Wide/Critical systems below this count as understaffed
```

(Everything under `provisioning:` is optional and covered under "Arming
provisioning" below.)

## Step 4 — Boot and verify (POC-local)

```bash
ASGARD_DATABASE_URL=postgres://postgres:change-me@localhost:5432/asgard \
ASGARD_SECRET_KEY=$ASGARD_SECRET_KEY \
asgard serve --bind 0.0.0.0:8080 --config ./asgard.yaml
```

On first boot with no `ASGARD_ADMIN_PASSWORD`, the log prints a generated admin
username + password **once**. Grab it, then:

1. `curl -fsS http://localhost:8080/healthz` → `ok`.
2. Open `/` in a browser → you get the sign-in screen. Log in with the admin
   credentials. (Set `ASGARD_ADMIN_PASSWORD` to control the password on future
   boots; change it after first login.)
3. Confirm the human surface is enforced: `curl -i http://localhost:8080/api/projects`
   with no session → `401`.

You now have a working, governed control plane. Onboard a project from the
dashboard (or via the agent runbook), mint a virtual key, and point an MCP client
at it (next step).

## Step 5 — Connect an MCP client

The MCP server is at `https://<host>/mcp` (Streamable HTTP). Authenticate with a
**project virtual key** as a bearer token. With the MCP Inspector or any
Streamable-HTTP client:

- URL: `https://<host>/mcp`
- Header: `Authorization: Bearer <project virtual key>`

`initialize` negotiates, `tools/list` shows the catalog (`list_services`,
`register_project`, `request_resource`, `seed_plan`, the `cost_*` tools, …), and
project-scoped tools act on the **key's** project — a different `project_id`
argument is denied. Mint the key from the dashboard (Projects → a registered
project → mint key) or with `POST /api/projects/{id}/keys`.

### Agent-seed over MCP

Agents bootstrap a repo's guidance through the seed tools. `seed_plan` takes the
repo's languages plus a description of the work and returns the **minimal**
relevant set of files (core operating agreement + per-language add-ons + matching
domain overlays + relevant templates), not a one-shot dump; `seed_get` returns
each file's body and the path to write it to. This is how a repo opts into your
standards without a human curating the file list.

---

## Enterprise: OIDC / SSO (rung 2)

Asgard uses the OIDC **authorization-code flow** and reads the user's profile from
the IdP's `/userinfo` endpoint (no local JWT/JWKS validation — lower operational
risk). Configure it with env vars; when `ASGARD_OIDC_DOMAIN` is set, the
`Sign in with SSO` button appears on the login screen and `/api/auth/oidc/*`
becomes active.

```bash
ASGARD_OIDC_DOMAIN=your-tenant.us.auth0.com          # endpoints derived from this
ASGARD_OIDC_CLIENT_ID=...
ASGARD_OIDC_CLIENT_SECRET=...
ASGARD_OIDC_REDIRECT_URI=https://<host>/api/auth/oidc/callback
# ASGARD_OIDC_SCOPES defaults to "openid email profile"
```

In your IdP, create a **Regular Web Application** for login:

- Allowed callback URL: `https://<host>/api/auth/oidc/callback` (must match
  `ASGARD_OIDC_REDIRECT_URI` exactly).
- Grant: authorization code. Scopes: `openid email profile`.

The local admin still works as a break-glass account alongside SSO. Live callback
URL / audience tuning is expected in-environment iteration — if the callback
fails, the most common cause is a mismatched redirect URI.

## Enterprise: arming provisioning

Out of the box, provisioning is **unarmed** (the catalog is discoverable and the
dry-run path works, but nothing real is created). To arm it:

1. Add a `terraform` block to `asgard.yaml` pointing at the bundled modules. The
   official container ships `terraform` on `PATH` and the modules at `/modules`,
   so no mount is needed — just point `modules_dir` there:

   ```yaml
   provisioning:
     terraform:
       modules_dir: /modules         # bundled in the image (or your own mounted tree)
       work_dir: /data/asgard-tf     # persisted working dirs + local TF state
     # Allow only the targets you intend to provision into:
     allowed:
       - { cloud: auth0, account: your-tenant }
   ```

   (Running from source instead of the container? Point `modules_dir` at the
   repo's `modules/` directory and ensure `terraform` is on `PATH`.)

2. **Auth0 provisioning** (the `auth0-application` service) uses the Terraform
   Auth0 provider, which reads **M2M Management API credentials from the
   environment**. The connector spawns `terraform` as a child process that
   inherits Asgard's environment, so setting these on the Asgard process is
   sufficient:

   ```bash
   AUTH0_DOMAIN=your-tenant.us.auth0.com
   AUTH0_CLIENT_ID=...        # a Machine-to-Machine app authorized for the Management API
   AUTH0_CLIENT_SECRET=...
   ```

   So the enterprise path uses **two Auth0 apps**: a Regular Web App for human
   login (above) and an M2M app for provisioning (here).

3. AWS provisioning is the same Terraform path; keep it unarmed for a first
   deploy and add the AWS target only when you are ready. Cost Explorer reads are
   independent of provisioning and can be enabled on their own.

Provisioned secret values (e.g. an Auth0 client secret) are stored as a
`secret_ref` in the encrypted secret store and surfaced only over the
project-key-gated `get_secret` MCP tool — never in the resource record, the DB in
plaintext, or the audit log.

---

## Environment variable reference

| Variable | Purpose | Default |
|---|---|---|
| `ASGARD_DATABASE_URL` | `sqlite://…` or `postgres://…`. Migrations run on boot. | `sqlite://asgard.db` |
| `ASGARD_BIND` | Listen address. | `0.0.0.0:8080` |
| `ASGARD_SECRET_KEY` | 64 hex chars (32 bytes) for the secret store. From your KMS. | dev key (insecure) |
| `ASGARD_ADMIN_USER` | Initial admin username. | `admin` |
| `ASGARD_ADMIN_PASSWORD` | Initial admin password. If unset and no admin exists, one is generated + logged once. | (generated) |
| `ASGARD_OIDC_DOMAIN` | IdP domain; presence enables SSO. Endpoints derived as `/authorize`, `/oauth/token`, `/userinfo`. | (off) |
| `ASGARD_OIDC_CLIENT_ID` / `_SECRET` / `_REDIRECT_URI` | OIDC web-app credentials + callback. | — |
| `ASGARD_OIDC_SCOPES` | Space-separated scopes. | `openid email profile` |
| `ASGARD_DEV_INSECURE` | `1`/`true` disables human-session enforcement. Loopback-only; ignored otherwise. | off |
| `ASGARD_FORCE_HTTPS` | `1`/`true` forces `Secure` on auth cookies regardless of detected scheme — "HTTPS is required." Set this when TLS is mandatory everywhere. | off (adaptive) |
| `AUTH0_DOMAIN` / `AUTH0_CLIENT_ID` / `AUTH0_CLIENT_SECRET` | M2M creds passed through to the Terraform Auth0 provider when provisioning is armed. | — |
| `ASGARD_GIT_TOKEN` | Token for catalog source repos (GitHub/GitLab), if configured. | — |
| `ASGARD_GUARDRAIL_MODE` | `enforce` (default) or `monitor`. | `enforce` |

Provider keys for inference backends (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`)
activate the corresponding inference modules when present; see
[Inference backends](./inference-backends.md).

---

## Operational notes

- **Probes.** `GET /healthz` is **liveness** (static `ok`, touches nothing).
  `GET /readyz` is **readiness** — it confirms the database is reachable and
  returns `503` if not. Point your orchestrator's readiness probe at `/readyz`,
  liveness at `/healthz`. The container's `HEALTHCHECK` uses `/readyz`.
- **Graceful shutdown.** On `SIGTERM`/Ctrl-C the server stops accepting new
  connections and drains in-flight requests before exiting. Combined with the
  single-replica model, give the process a few seconds to stop.
- **Cookies.** The session cookie is `HttpOnly; SameSite=Lax`, and `Secure`
  **only when the request arrived over TLS** (`X-Forwarded-Proto: https`). So a
  plain-http deployment works out of the box, and once you put TLS in front the
  cookie is automatically `Secure` and never crosses a plaintext hop — no config
  flag to flip. Enterprises that mandate TLS everywhere can set
  **`ASGARD_FORCE_HTTPS=1`** to force `Secure` on unconditionally (so a cookie can
  never be issued non-`Secure`, even if a misconfigured proxy drops the header).
- **CORS.** There is no permissive CORS layer — the dashboard is same-origin and
  API/MCP consumers aren't browsers, so cross-origin browser access is denied by
  default. If you front the API from a different origin, that's a deliberate
  change to make.
- **Login throttling.** Local sign-in is rate-limited per source IP
  (`X-Forwarded-For`): repeated failures lock that source out for a few minutes.
  It's best-effort and in-memory (per replica) — Argon2 already makes each attempt
  expensive. Prefer SSO (rung 2) for the human surface in an enterprise setting.

## Troubleshooting

- **Dashboard returns 401 for everything.** Expected when not signed in. Log in at
  `/`. If you can't, check the boot log for the generated admin password, or set
  `ASGARD_ADMIN_PASSWORD` and restart.
- **`/mcp` returns 401.** The bearer token must be a valid, active **project
  virtual key**, not a human session token. Mint one for a registered project.
- **`/mcp` returns 404.** You're hitting the wrong path or method — it's `POST`
  (and `GET`/`DELETE`) on exactly `/mcp`.
- **OIDC callback fails / `state mismatch`.** The redirect URI registered in the
  IdP must equal `ASGARD_OIDC_REDIRECT_URI` exactly (scheme, host, path). The
  state cookie is short-lived; don't reuse a stale callback URL.
- **Armed Auth0 provisioning fails with auth errors.** Confirm the `AUTH0_*` M2M
  variables are set **on the Asgard process** (the Terraform child inherits them)
  and the M2M app is authorized for the Management API scopes the module needs.
- **`ASGARD_DEV_INSECURE=1` had no effect.** It's only honored on a loopback bind
  (`127.0.0.1`/`localhost`/`::1`); on any other bind it logs a warning and stays
  off.

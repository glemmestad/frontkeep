---
sidebar_position: 2
title: Quickstart
---

# Quickstart

Asgard is one static binary. The default path needs **only a Git token** and
SQLite — no Postgres, no Redis, no Kubernetes.

## Run it

```sh
# Docker (SQLite, embedded UI)
docker run -p 8080:8080 -e ASGARD_GIT_TOKEN=ghp_xxx ghcr.io/asgard/asgard:latest

# or from source
cargo run -p asgard -- serve --database-url sqlite://asgard.db
```

Open `http://localhost:8080` for the UI (catalog discovery, cost, audit-trace
viewer, kill switch). REST is under `/api`, GraphQL at `/graphql`, and the MCP
server is at `/mcp`.

> **Using a running Asgard from your agent?** See
> [Connect an agent (MCP)](./connect-agent.md) — how to point Claude Code,
> Cursor, or the MCP Inspector at `/mcp` with a project key and start building.
> To stand an instance up, see the [operator deploy guide](./deploy.md).

## Ingest a catalog

Point Asgard at repos containing `agent.yaml` / `eval.yaml` / `mcp.yaml` … via
`asgard.yaml`:

```yaml
sources:
  - { provider: github, owner: your-org, repo: your-repo, ref: main }
```

Entities reconcile in on startup and on an interval; removing a manifest from a
repo removes its entity (deletes propagate).

## The lightweight proof (reproduces `scripts/e2e.sh`)

```sh
# 1. mint a per-project gateway key
curl -XPOST localhost:8080/api/projects/proj-2026-0001/keys \
  -d '{"budget_usd":100,"data_class":"internal"}'

# 2. route a completion through the gateway (cost is attributed to the project)
curl -XPOST localhost:8080/api/gateway/chat -H "authorization: Bearer asg_…" \
  -d '{"model":"model:default/mock","messages":[{"role":"user","content":"hi"}],"data_class":"internal"}'

# 3. see the spend
curl localhost:8080/api/projects/proj-2026-0001/usage

# 4. kill the project — the next gateway call is rejected
curl -XPOST localhost:8080/api/projects/proj-2026-0001/kill
```

A wrong data-class/model pairing returns `403` (policy), a leaked secret returns
`400` (guardrail), and every call is in the audit trail with its trace id.

## Scale out

Switch to Postgres and add stateless replicas — same binary, identical behavior:

```sh
asgard serve --database-url postgres://user:pass@host/asgard
```

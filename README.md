<h1 align="center">Asgard</h1>

<p align="center"><b>An open-source control plane for AI &amp; agent development inside a company.</b><br/>
A single static Rust binary: a manifest-driven service catalog + model gateway + policy + cost attribution + registry + audit — exposed over CLI, MCP, REST + GraphQL, and an embedded UI.</p>

---

## Why

Backstage answers *"what exists and who owns it."* For agents that isn't enough. You also need to know **is it safe, is it working, what is it costing, and what did it just do** — and you need every capability reachable by an agent, not just a human clicking a UI.

Asgard is **agents-first**. The thing it's built around is a **governed onboarding loop**: point a new effort's AI agent at a seed repo → it reads the company standards → **registers the project (the mandatory gate)** → that unlocks **provisioning of real services through the orchestrator** → and every bit of model and infrastructure spend is **attributed by project / owner / manager / group**. It's a **hub, not a workflow engine**: Asgard serves guidance and data and mints credentials; the agent acts.

## What you get

- **Service catalog (manifest-driven).** Everything an agent can provision is one YAML per service (`services/<id>/service.yaml`) declaring how it's provisioned (`provisioner.connector`) and how its cost is attributed (`cost.source.type`). Adding a service is dropping a manifest + a Terraform module — **no recompile**. The `terraform` connector is the universal path (any cloud); `exec`/`http`/`litellm` cover the rest.
- **Registry — the gate.** Nothing provisions or spends until a project is registered and active. Registration mints a stable `proj-YYYY-NNNN` id and supplies the attribution dimensions stamped onto every usage event. Lifecycle: classification tiers, evidence-gated promotion/demotion, review-date sweeps.
- **Gateway.** Every model call routes through it: per-project virtual keys, budgets, model allowlists per data-classification, PII/secret/prompt-injection guardrails, full audit. **OpenAI + Anthropic built in** (Mock by default, offline); **any OpenAI-compatible backend — LiteLLM, Databricks Model Serving, vLLM — is an enableable module**, not core code. LiteLLM also supports **per-project virtual keys** as a governed, budgeted, cost-attributed resource.
- **Policy.** One Cedar engine queried by gateway, catalog, workflow, and runtime: *can this principal do this, against this data class, with this model — and does it need approval, from whom?*
- **Cost.** Model spend and infrastructure spend both denormalize owner/manager/group, so attribution is a plain query. Daily rollups, month-to-date deltas, EOM forecast, an org cost tree, and a governed cost Q&A.
- **Knowledge platform.** Normative **standards**, advisory **guidance**, and **recipes** (composable runbooks) — versioned with an edit trail and per-version diff, full-text search, moderation (draft → approve), and category facets. Served to humans in the UI and to agents over MCP.
- **MCP control plane.** Agents discover services, register projects, fetch credentials, read standards/guidance/recipes, and request resources — all as MCP tools. The remote server lives at `/mcp`; auth is a user PAT (acts across every project you own/manage) or a per-project key.
- **Surfaces.** CLI, MCP, embedded Web UI, REST + GraphQL.

## Design principles

- **One static binary.** Rust, embedded UI. `docker run asgard` and you're productive in an afternoon.
- **SQLite by default, Postgres opt-in.** Identical behavior on both — the SQLite path needs zero external services; switch `--database-url` to Postgres to scale out.
- **Agents first, humans second.** Every capability is an MCP tool, not just a UI/REST route.
- **Hub, not world-builder.** Serve guidance and data, mint credentials, let the agent act. Resist bespoke orchestration.
- **Open core, honest seams.** The governance spine is OSS. Enterprise features (SAML/SCIM, multi-tenant, SIEM streaming) sit behind clean trait seams.

## Quickstart

```sh
# SQLite, no external services, no login on loopback — browse the UI at http://localhost:8080
cargo run -p asgard -- serve --database-url sqlite://asgard.db
# (set ASGARD_DEV_INSECURE=1 to skip auth on loopback while exploring)
```

Then connect an agent to the **MCP front door** — the Getting Started tab in the UI mints a Personal Access Token and shows the exact snippet for Claude Code, Codex, and Cursor. Or drive it from the CLI:

```sh
asgard project register --name "My Service" --owner you@corp.example \
  --manager you@corp.example --group platform --classification poc   # the gate; mints proj-YYYY-NNNN
asgard gateway login --project proj-2026-0001                        # get the project's virtual key
asgard catalog ls                                                    # what exists
asgard project cost --by group                                       # spend rolled up by dimension
asgard validate services/s3-bucket/service.yaml                      # offline manifest check
```

See [`docs/docs/`](docs/docs/) for the [onboarding loop](docs/docs/onboarding-loop.md), [connecting an agent](docs/docs/connect-agent.md), [deploying Asgard](docs/docs/deploy.md), [inference backends](docs/docs/inference-backends.md), and the [architecture](docs/docs/architecture.md).

## Status

Pre-1.0. The OSS governance core is the focus; architecture decisions are recorded in [`RFC-0001`](RFC-0001-entity-model.md) and [`RFC-0002`](RFC-0002-policy-and-sandbox.md), and the prioritized backlog lives in [`plans/ROADMAP.md`](plans/ROADMAP.md).

## License

Apache 2.0. Contributions under the [CLA](CLA.md). See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).

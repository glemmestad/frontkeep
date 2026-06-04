# Workflow

How an AI agent (or a human) should actually do work in a repository that has adopted the Asgard seed. Read this when you are about to make changes. If you are only reading the catalog from somewhere else, you don't need this file — read [`../AGENTS.md`](../AGENTS.md).

## Before you start

1. Read [`../AGENTS.md`](../AGENTS.md) so you understand the model and the MCP tools.
2. Call `list_standards` + `get_standards` (or read [`STANDARDS.md`](STANDARDS.md) offline). Your output must conform.
3. If your task touches auth, secrets, data classification, model selection, or anything network-facing, read [`SECURITY.md`](SECURITY.md) first.
4. Examine the existing manifests, code, and tests in this repo before changing anything.

## Branching

- Branch from the default branch. Name: `<your-handle>/<short-topic>` (e.g. `platform/add-rag-eval`).
- Keep diffs narrow and reviewable — one logical change per branch.
- Conventional-commit PR titles (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`).

## Register the project (mandatory gate)

Before you request any chargeable resource — a gateway key, a model call, a sandbox run, shared storage — the project must be registered.

- Check `AGENTS.md` for an existing `proj-YYYY-NNNN` id. If it's there, the project is already registered; reuse it.
- If not, call **`register_project`** with the project name, owner, and the data classification you'll be handling. It returns a stable `proj-YYYY-NNNN` id. Record that id back in `AGENTS.md`.

Asgard will refuse to issue resources to an unregistered project. This is by design: every resource carries a project owner so cost is attributed and every action is auditable. There is no anonymous spend.

## Request shared capabilities through Asgard

Don't wire providers, storage, or secrets directly. Use the catalog and the MCP tools:

1. **Discover.** `catalog_search` (by `kind` and/or `query`) and `catalog_get` to find an existing model, tool, dataset, or agent you can reuse.
2. **Provision.** `request_resource` for shared infrastructure your project needs (storage, a secret, access to a tool/MCP server). It returns a reference your code uses; secret *values* are fetched at runtime, never committed.
3. **Get a key.** `gateway_credential` mints a per-project virtual key for the model gateway.
4. **Call models.** `gateway_chat` invokes an allowed model through the gateway, with the project's budget, the data-class × model policy, guardrails, audit, and kill switch all applied. Never call a provider SDK directly.
5. **Watch spend.** `cost_report` shows the project's attributed spend. Check it before and after expensive work.

If a capability you need isn't in the catalog, stop and ask a human — don't improvise around it.

## While editing

- Stay surgical. Don't refactor working code you weren't asked to touch, and don't reformat on the way through.
- Match the existing tone and conventions in the repo.
- Keep entity manifests valid — they're validated against their JSON Schemas at ingestion, and an invalid manifest is surfaced as a PR comment, not silently dropped.
- If you change what an entity does, update its manifest in the same change; drift between code and manifest is a defect.

## The eval and merge gate (definition of done)

Where an `Eval` suite gates this repo, it runs on every PR, posts a scored verdict as a PR comment, and blocks merge on failure. A change is **done** when:

- The requested behavior is implemented and covered by a test.
- Format, lint, type check, and unit tests pass — and you actually ran them.
- The eval gate, where one applies, passes. **Never lower an eval threshold to make a failing verdict green.** A regression in the verdict is a signal to fix the change, not the threshold.
- Affected documentation and manifests are updated.
- The PR description explains the "why" and references what prompted the change.
- The diff is as small as it can be.

## Verify before you claim done

After implementing, prove it works: run the tests, exercise the path end to end (hit the endpoint, run the invocation, read the `cost_report`, confirm the action shows in the audit trail). Do not report success on something you didn't run.

## When to stop and ask

Surface a question to a human (don't silently decide) when:

- The work touches `restricted` data, or data above the project's registered class.
- The work would promote the project to a higher maturity/criticality tier.
- You need a model not allowlisted for your data class, or a capability not in the catalog.
- You'd have to bypass the gateway, weaken a policy, or disable a guardrail to proceed.
- A `register_project` or `request_resource` call fails for a reason you can't resolve.

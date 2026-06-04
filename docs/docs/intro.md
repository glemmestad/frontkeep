---
slug: /
sidebar_position: 1
title: Introduction
---

# Asgard

**An open-source control plane for AI & agent development inside a company.**

Backstage answers *"what exists and who owns it."* For non-deterministic agent
systems that isn't enough — you also need to know **is it safe, is it working,
what is it costing, and what did it just do.** Asgard adds that governance
overlay — gateway, evals, audit, cost, policy, kill switches — as the spine, not
a plugin.

## The six load-bearing components

1. **Catalog** — a typed entity graph (`Agent`, `Prompt`, `Tool`/`MCPServer`,
   `Model`, `Dataset`, `Eval`, `Project`, plus Backstage-compatible kinds).
   Source of truth is YAML in Git, reconciled into the store. Pull-based, so
   deletes propagate.
2. **Gateway** — every model call routes through it: per-project virtual keys,
   budgets, model allowlists per data-classification, PII/secret/prompt-injection
   guardrails, full audit with a propagated `x-asgard-trace-id`, and a kill switch.
3. **Policy** — one Cedar engine queried by gateway, catalog, workflow, and
   runtime: *can this principal do this, against this data class, with this model,
   and does it need approval?*
4. **Workflow + approvals** — request → approve → fulfill state machine.
5. **Evals** — eval suites that gate PR merges. CI for non-determinism.
6. **Runtime / sandbox** — ephemeral isolated execution with per-invocation
   budget/step/wall-time caps enforced at the runtime (gVisor / container / local
   backends behind one trait).

Around these sits the **registry gate** (no provisioning or spend until a project
is registered), a **manifest-driven service catalog** an agent provisions through,
and a **knowledge platform** — versioned, searchable standards, guidance, and
recipes, served to humans in the UI and to agents over MCP.

## Design principles

- **One static binary.** Rust, embedded UI. `docker run asgard` with a Git token.
- **SQLite by default, Postgres opt-in.** Identical behavior on both; the same
  binary scales out by switching `--database-url` and adding stateless replicas.
- **Kubernetes supported, never required.** Headline paths are `docker run` and
  systemd; a Helm chart and Terraform module ship too.
- **Open core with honest seams.** The governance core is OSS; enterprise
  features (SAML/SCIM, multi-tenant, SIEM streaming) sit behind clear trait seams.

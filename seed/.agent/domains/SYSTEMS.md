# Systems / backend-service domain overlay

Pulled in when the work is a backend service, API, or infrastructure-facing
component handling real traffic and state.

## Contracts & compatibility
- Treat the API/schema as a contract. Additive changes are safe; breaking changes need a version and a migration path. Never silently change response shapes.
- Migrations are forward-only and reversible-in-practice: write the migration, test it on a copy, and make sure the old and new code can both run during a rollout.

## Failure is the default case
- Every external call (DB, network, queue) can fail, time out, or return garbage. Set timeouts, bound retries with backoff, and make operations idempotent where a retry could duplicate work.
- Don't lose data on partial failure. Make state transitions atomic or compensatable.

## Observability
- Emit structured logs with a correlation/trace id, plus the metrics that would let you debug this at 3am. Log decisions and errors, not noise.

## Security & resources
- Validate and bound all input (size, rate, shape). Least privilege for credentials and network access.
- Watch resource lifecycles: close connections, cap pool sizes, avoid unbounded queues/caches. A leak under load is an outage.

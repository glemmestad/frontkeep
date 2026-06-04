# Data pipelines domain overlay

Pulled in when the work moves, transforms, or stores data at scale (ETL/ELT,
batch/stream jobs, analytics).

## Idempotency & reprocessing
- A pipeline run must be safe to re-run: partition by a key + window so a replay overwrites rather than duplicates. Assume jobs will be retried and backfilled.
- Make each stage's output deterministic given its input and code version.

## Data quality is part of done
- Validate at ingestion: schema, types, nullability, ranges. Quarantine bad records instead of silently dropping or crashing the whole run.
- Track row counts and key metrics across stages so a drop or explosion is caught, not discovered downstream.

## Provenance & cost
- Record where data came from, its classification, and its retention rule. Confidential data carries its handling rules through every stage.
- Mind the cost of full scans and wide shuffles; partition and prune. Attribute storage/compute to the owning project (register it, like any other resource).

## Schema evolution
- Evolve schemas additively with defaults; coordinate producer and consumer changes. Never break a downstream consumer without a migration window.

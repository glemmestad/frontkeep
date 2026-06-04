# Unity Catalog Volume

Provisions a `databricks_schema` + `databricks_volume` via the terraform connector
+ the Databricks provider. Auto-approvable (cheap, self-service).

**Spec fields**: `name` (volume name), `catalog_name` (an existing UC catalog),
optional `schema_name` (defaults to the project id). The `project` tag is recorded
in the schema properties for attribution.

**Outputs**: `volume_path` (`/Volumes/<catalog>/<schema>/<name>`), `schema_full_name`.

> UC storage isn't billed as per-resource DBUs, so this carries no live cost
> source (estimate `$0`); compute that reads/writes it is billed on the warehouse
> or job that does so.

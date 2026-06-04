# TypeScript / JavaScript add-on

Language conventions layered on top of `.agent/STANDARDS.md`. Pulled in when the
repo contains TypeScript or JavaScript.

## Build & checks
- The done bar: `tsc --noEmit` clean, the linter (ESLint/Biome) clean, the formatter (Prettier/Biome) applied, and the test runner (Vitest/Jest) green.
- `strict: true` in `tsconfig`. New code is TypeScript, not JavaScript, unless the repo is deliberately JS.

## Idioms
- No `any`. Reach for `unknown` + narrowing, generics, or a real type. `as` casts need a reason.
- Prefer `const`, immutable data, and pure functions. Model nullability explicitly (`T | null`) and handle it; don't lean on `!`.
- Validate external/untrusted data at the boundary with a schema (`zod` or equivalent) and infer the type from the schema — don't hand-write a type that can drift from the validator.
- `async/await` over raw promise chains; never leave a promise unawaited (no floating promises).

## Dependencies & supply chain
- Lockfile committed; install with `--frozen-lockfile` in CI. Audit new dependencies — npm's transitive surface is large and a common attack vector.
- Don't pull a framework or utility lib for something the standard library / a few lines already do.

## Frontend
- If this is UI work, also pull the `frontend` domain overlay for accessibility and browser-verification expectations.

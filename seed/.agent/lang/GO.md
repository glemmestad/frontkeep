# Go add-on

Language conventions layered on top of `.agent/STANDARDS.md`. Pulled in when the
repo contains Go.

## Build & checks
- The done bar: `gofmt`/`goimports` clean, `go vet` clean, a linter (`golangci-lint`) clean, and `go test ./...` green (with `-race` for anything concurrent).

## Idioms
- Handle every error explicitly; wrap with `fmt.Errorf("...: %w", err)` to preserve the chain. Never discard an error with `_` unless you say why.
- Accept interfaces, return structs. Keep interfaces small and defined at the consumer.
- Use `context.Context` as the first parameter for anything that does I/O or can be cancelled; thread it through, don't store it in a struct.
- Concurrency: a goroutine without a clear stop condition is a leak. Use channels/`sync` deliberately and run the race detector.

## Layout
- Follow the standard project layout; keep packages cohesive and avoid cyclic imports. Exported identifiers need doc comments.

## Dependencies
- `go.mod`/`go.sum` committed and tidy (`go mod tidy`). Justify new modules; prefer the standard library.

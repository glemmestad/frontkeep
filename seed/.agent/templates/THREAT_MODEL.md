# Threat model

Use for security-sensitive work (auth, secrets, untrusted input, agent tool-use,
network boundaries). Short and specific beats exhaustive.

## What we're protecting
<Assets: data classes, credentials, capabilities. Who must not reach them.>

## Trust boundaries
<Where untrusted data/users cross into trusted code. Inputs and their source.>

## Threats considered
| Threat | Vector | Mitigation in this change |
|---|---|---|
| <e.g. injection> | <how> | <what stops it> |

## Decisions
<Least-privilege choices, what is validated where, what is intentionally out of scope and why.>

## Residual risk
<What remains, and whether a human needs to sign off.>

# Compilers & language tooling domain overlay

Pulled in when the work is a compiler, interpreter, parser, IR, codegen, or an
optimization pass. The bar is exactness: a compiler bug doesn't crash, it silently
miscompiles, and the symptom shows up far from the cause.

## Semantics are the contract
- Define the language/IR semantics before you touch a pass. A transformation is correct only if it preserves observable behavior — write down what "observable" means here.
- An optimization that changes results is a miscompile, not a speedup. Prove (or test hard) that each pass is semantics-preserving, including on edge cases: overflow, NaN, aliasing, undefined behavior, empty inputs.
- Order matters. State the assumptions a pass makes about the IR coming in (SSA form, normalized control flow) and the invariants it guarantees going out.

## Parsing & front end
- Report errors with source location and a useful message; recover where you can so one error doesn't mask the rest. A parser that dies on the first mistake is hostile to use.
- Round-trip where it makes sense: parse → print → parse should be stable. Fuzz the parser — malformed input must produce a diagnostic, never a panic or a hang.

## IR & passes
- Keep the IR validatable: write a verifier that checks structural invariants, and run it between passes in debug builds. Most "optimizer bugs" are an earlier pass leaving the IR malformed.
- Each pass is small, named, and independently testable. A pass that does three things hides which one broke.

## Testing
- Golden tests on IR/output for representative programs, plus differential testing: compare against a reference implementation or `-O0` vs optimized output on the same inputs.
- Fuzz with random valid programs and check that optimized and unoptimized runs agree. This catches the miscompiles unit tests miss.

## Done bar
A compiler change is done when each pass's pre/post invariants are stated and
verified, semantics-preservation is demonstrated (golden + differential tests),
the parser handles malformed input without crashing, the IR verifier passes
between stages, and you've tested the ugly edges (overflow, aliasing, empty
input), not just the happy path.

# Definition of done

A change is done when it is verified, safe, and reviewable — not when the code
compiles. Apply this bar before you hand work back. When in doubt, the work is
not done.

## The gates (all green, actually run)
- Format and lint pass with the language's standard tools. No suppressed lints without a one-line reason.
- Type checks pass on the code you changed. Treat type errors as build failures for new code.
- Tests pass, and there is a test for the new behavior. A change without a test for what it changed is not done. Never weaken or skip a test to go green.
- The relevant eval gate passes for agent/prompt/model work.

## Verification, not assertion
- Run the thing. Invoke the function, hit the endpoint, exercise the path you changed, and say what you observed. "It should work" is not verification.
- Never claim a check, test, or eval passed unless you actually ran it and saw it pass.
- If you couldn't verify something, say so explicitly and name what is unverified.

## Leave it clean
- Remove code, imports, variables, and files that *your* change orphaned. Don't leave dead branches behind a flag you added.
- Don't touch adjacent code that isn't part of the task. Stay surgical: every changed line traces to the work.
- No narrative comments, no commented-out code, no debug prints left in.

## Security and data class
- If the change touches auth, secrets, data classification, model selection, or anything network-facing, re-read `.agent/SECURITY.md` and confirm you didn't widen the blast radius.
- No secret landed in code, config, or history. Every model call still goes through the gateway. No grant got broader to silence an error.
- If the work pushed the project toward a higher data class or maturity tier, stop and get a human — that is not yours to decide.

## Hand-back
- Produce a change summary (see `.agent/templates/CHANGE_SUMMARY.md`): what changed, why, how you verified it, blast radius, follow-ups.
- State remaining risk and anything you deliberately deferred, with the reason.

## Done bar
Done means: gates green (and run), behavior verified by exercising it, no orphaned
code, security/data-class checks clear, change summary written. If any line here is
unmet, the work is in progress, not done.

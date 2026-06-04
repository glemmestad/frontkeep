# Formal verification domain overlay

Pulled in when the work involves proofs, model checking, SMT, theorem provers,
or specification languages (TLA+, Coq, Lean, and friends). The whole value is
trust, so the failure mode that matters most is a proof that passes while proving
the wrong thing.

## The spec is the deliverable
- Write the specification before the proof, and write it so a human can read what it claims. A verified system is only as trustworthy as the property you stated.
- State the assumptions explicitly: what's in the trusted base, what's modeled as atomic, what's out of scope. An unstated assumption is an unproven one.
- Guard against vacuous truth. A property that holds because its precondition is never reachable proves nothing — check that the interesting states are actually explored.

## Proofs and checks
- No admitted lemmas, `sorry`, `Admitted`, or assumed axioms left in a "complete" proof. If something is assumed, it's flagged loudly, not buried.
- For model checking, state the bound (depth, state count) and whether the check was exhaustive or bounded. A bounded check that found no counterexample is not a proof of correctness — say which you have.
- When the prover reports unknown/timeout, that is not a pass. Treat it as an open obligation, not a green check.

## Keep the model honest
- The proof is about the model, not the code. State how the model relates to the implementation, and where they can diverge (the gap is where bugs live).
- Re-run the full proof/check from clean; don't trust a cached result. Pin the prover/solver version — proofs are sensitive to it.

## Done bar
A formal-verification change is done when the property is stated in human-readable
terms, all assumptions and the trusted base are written down, there are no
admitted/`sorry`/assumed obligations, bounded vs. exhaustive is declared, vacuity
is ruled out, the model-to-implementation gap is documented, and the proof
re-runs clean against a pinned prover version.

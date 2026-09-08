# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 62 — reproducible action-temporal verification jobs

**Status: integration candidate complete.**

Milestone 62 extends the heterogeneous verification-job dispatcher with a fifth explicit family, `analysis "action-temporal"`, without changing the historical implicit multi-response default or the sealed safety, exact-state, and proposition-expression job contracts.

Action-temporal property files reuse the existing M18 textual grammar exactly: `response("trigger","response")` and `infinitely-often("action"[, ...])`. The job layer does not implement temporal semantics. It parses the existing specification, canonicalizes it through the existing frontend, and delegates execution to `check_action_temporal_with_fairness_profile_and_limits`.

The family preserves the canonical backend identity exposed by the frontend: response specifications report `backend:"response"`; recurring-action specifications report `backend:"buchi"`. Machine evidence contains only frontend model-state/action traces. Pending-bit vectors, monitor control state, and Büchi automaton control state remain internal implementation details and are not leaked into the heterogeneous job schema.

Model and product state/transition/depth budgets reuse the sealed staged temporal APIs. A blocking model cutoff remains model-stage `INCONCLUSIVE`; a blocking product cutoff remains product-stage `INCONCLUSIVE`. Real finite or lasso violations already justified before a later cutoff remain conclusive. No prefix gap is promoted to proof.

Weak and strong exact-action fairness declarations reuse the existing `FairnessProfile`. No-fairness remains the default. Weak/strong overlap is canonicalized to the strong class by the existing fairness authority, and fairness continues to constrain infinite executions only.

Structured results use heterogeneous schema v2 with `analysis:"action-temporal"`, stable backend identity, canonical property text, explicit fairness sets, independent model/product limits and accounting, exact cutoff provenance, and normalized finite/lasso evidence. Satisfied jobs use exit 0, temporal violations retain exit 10, incomplete verification uses exit 3, and malformed/configuration errors use exit 2.

Executable evidence includes manifest/parser round trips; response and Büchi backend identity; direct staged frontend differential accounting; no-fair lasso normalization; weak fairness; weak/strong overlap canonicalization; model/product cutoff provenance; malformed temporal input; built `fvlab` JSON/exit integration; and five-family raw/expectation suites that preserve direct nested job envelopes, declaration order, aggregate precedence, deterministic repetition, and built `fvlab` / `fvlab-suite` equivalence across legacy multi-response, safety, exact-state, proposition-expression, and action-temporal jobs.

The implementation candidate `632c29dcf2030b7c4c19db1628fa76c9a342e5b1` passed CI #521: rustfmt, all-target build, Clippy with `-D warnings`, the complete test suite, and every historical CLI regression gate. The same exact candidate passed Bounded state-property CLI workflow #371. This status update itself still requires exact-head CI before merge.

M62 adds no new temporal operator, arbitrary LTL/CTL compilation, traversal engine, fairness semantics, fairness-by-default behavior, shell execution, wall-clock proof bound, generic plugin/RPC subsystem, or performance/security claim.

## Next frontier — Milestone 63: declarative bounded deadlock policies and reproducible jobs

M62 completes reproducible job coverage for the existing external action-temporal language. The highest-value remaining external verification gap is deadlock policy: M7 has a trusted deterministic deadlock backend, but legitimate-terminal policy is still supplied as a Rust predicate and `check_deadlock` is exhaustive-only. Declarative models and M22 Boolean propositions therefore cannot yet express a portable policy such as “terminal states are legitimate when `done or cancelled`”, and a resource cutoff cannot yet be propagated honestly through deadlock verification.

Milestone 63 should close that gap as one coherent backend-to-job vertical slice rather than adding a wrapper-only family.

Acceptance criteria:

- add bounded deadlock verification on the canonical BFS substrate with state/transition/depth limits and explicit `INCONCLUSIVE`; a real unexpected terminal found before a later cutoff may conclude `DEADLOCK_FOUND`, while `DEADLOCK_FREE` requires exhaustive completion;
- preserve deterministic shortest unexpected-terminal witnesses, exact accounting, canonical limit precedence, and the invariant that partial exploration never fabricates a terminal;
- add a typed/textual declarative deadlock-policy frontend whose legitimate-terminal predicate is an existing M22 Boolean proposition expression, resolving every proposition reference before backend execution and reusing the existing expression semantics rather than introducing another Boolean evaluator;
- expose the frontend against external declarative model files with model-space limits, native deadlock violation exit 5, inconclusive exit 3, and malformed/reference/configuration exit 2;
- add explicit `analysis "deadlock"` verification jobs backed by the same frontend, rejecting temporal-only fairness and product limits before execution and preserving historical job families unchanged;
- extend schema-v2 machine results with model-space deadlock accounting/cutoff provenance and action/state shortest-witness evidence only; do not manufacture product accounting or temporal control state;
- extend raw and expectation suites to six families while preserving declaration order, aggregate precedence, complete nested envelopes, deterministic repetition, and direct built-binary equivalence;
- validate bounded deadlock semantics with an independent generated small-graph × legitimate-terminal-set × limit-profile oracle, including initial terminals, exact-bound completion, cutoff-before-witness, witness-before-later-cutoff, and no-false-terminal cases;
- add no fairness interpretation for finite deadlocks, symbolic-state engine, arithmetic state-expression language, second traversal engine, wall-clock proof bound, or performance/security claim.

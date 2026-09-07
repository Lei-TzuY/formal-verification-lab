# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 58 — expectation-aware regression suites

**Status: integration candidate complete.**

Milestone 58 extends the sealed M57 deterministic verification-suite surface with an explicit regression expectation contract while preserving raw suite behavior unchanged. Suite entries may optionally append `expect "satisfied|violated|inconclusive|error"`; historical `job "path.fvj"` entries remain valid and canonicalize exactly as before.

Expectation checking is explicit through `run_verification_suite_expectations_json` and `fvlab-suite <suite> --check-expectations --format json`. Expectation mode requires every suite entry to declare an expected outcome before any job is executed. Missing or unknown expectations therefore fail closed as suite-level errors instead of inferring intent from file names, prior runs, or process exit codes.

Every regression entry still executes through the sealed M56 structured job runner. The regression envelope records the declared expected outcome, the observed M56 outcome, a deterministic match/mismatch classification, and the complete observed schema-v1 job envelope unchanged. Expectations never edit, replace, recompute, or suppress model/property identity, fairness assumptions, configured budgets, accounting, cutoff provenance, finite traces, or lasso evidence.

All matching expectations produce regression outcome `matched` and exit `0`, including deliberately negative `VIOLATED`, `INCONCLUSIVE`, and structured job-level `ERROR` cases. Any number of mismatches are retained in suite order, produce regression outcome `mismatched`, and exit the dedicated code `13`. Suite-level syntax/loading/expectation-definition errors remain exit `2`. Raw M57 suite execution ignores expectations and retains its historical observed-outcome precedence and exits.

Executable evidence covers parser/backward compatibility, canonical round trips, unknown and missing expectation rejection, matched positive and negative outcomes, multiple ordered mismatches, direct-envelope preservation, deterministic repeated JSON, and raw-mode compatibility. Built-binary differential coverage executes both `fvlab-suite` and the sealed `fvlab temporal job ... --format json` path across malformed jobs, satisfied jobs, finite pending-terminal violations, lasso violations, product cutoffs, model cutoffs, and mixed weak/strong fairness. It verifies exact nested M56 envelope preservation for matching suites, exercises a deliberate mismatch for every one of those job classes, locks exit `13`, checks fail-closed incomplete expectations, and rejects malformed expectation-mode CLI usage.

The implementation candidate `264f1ec1587e41979604fac65ba925edb1eb4ce4` passed CI #482: rustfmt, all-target build, Clippy with `-D warnings`, the complete test suite, and every historical CLI regression gate. The same exact candidate passed Bounded state-property CLI workflow #332. Final documentation changes still require exact-head CI before merge.

M58 adds no witness-text golden matching, mutable baseline rewriting, automatic expectation rewriting, new verifier/fairness semantics, fairness-by-default behavior, concurrency/distributed execution, shell evaluation, wall-clock proof bounds, or performance/security claims.

## Next frontier — Milestone 59: heterogeneous verification jobs, declarative-safety vertical slice

M55–M58 make verification jobs reproducible, machine-readable, batchable, and expectation-aware, but one architectural limitation is now dominant: the job manifest is still specialized to the textual multi-response temporal path. The repository already has independently validated declarative Boolean safety semantics and proof-honest M24 bounded state-property execution, yet those existing engines cannot participate in the reproducible job/suite/regression infrastructure without a separate external command.

Milestone 59 should promote the job layer from a single temporal-property launcher into an explicit typed analysis dispatcher, beginning with one coherent cross-layer vertical slice for declarative Boolean safety. This is an integration milestone, not a new safety traversal.

Acceptance criteria:

- extend the verification-job manifest with an explicit, deterministic analysis/property-family discriminator while keeping every historical M55 multi-response manifest backward compatible;
- add declarative Boolean safety as the first non-temporal job family, compiling directly to the sealed M22/M23/M24 predicate/safety authorities rather than introducing another graph traversal or assertion semantics;
- preserve proof-honest state/transition/depth budgets and `INCONCLUSIVE` behavior for bounded safety jobs, including deterministic shortest falsifying witnesses and exact cutoff reasons/accounting;
- generalize the structured job result only as required to carry the existing safety evidence and identity without weakening or reinterpreting the current schema-v1 multi-response envelopes; if a schema revision is necessary, version it explicitly and keep old schema behavior readable/compatible;
- allow one M57/M58 suite to contain both historical multi-response jobs and the new safety jobs, with deterministic ordering, raw aggregate precedence, complete observed envelopes, and expectation matching across both families;
- add direct-backend and built-binary differential regressions for safe, violated, inconclusive, malformed, and mixed-family suites, including exact witness/cutoff preservation and historical M55–M58 compatibility;
- keep fairness meaningful only for temporal jobs and reject incompatible fairness directives on safety jobs instead of silently ignoring them;
- add no new fairness semantics, fairness-by-default behavior, second traversal engine, shell execution, wall-clock proof bound, performance/security claim, or generic plugin/RPC subsystem.

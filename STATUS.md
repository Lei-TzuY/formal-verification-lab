# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 59 — heterogeneous verification jobs

**Status: integration candidate complete.**

Milestone 59 promotes the sealed M55–M58 verification-job, machine-readable result, suite, and expectation infrastructure from a single multi-response temporal family to explicit heterogeneous analysis dispatch. Verification-job manifests may declare `analysis "multi-response"` or `analysis "safety"`; historical manifests that omit the directive remain effective multi-response jobs and preserve their canonical document and schema-v1 JSON behavior.

The declarative Boolean safety family composes the existing proposition-aware model document, Boolean proposition-expression parser, and proof-honest M24 bounded safety authority. It introduces no second state-space traversal. Safety jobs use model-space state/transition/depth budgets only, preserve deterministic shortest falsifying traces, return `INCONCLUSIVE` when a cutoff prevents proof, and reject temporal-only fairness directives or product-space limits before model/property execution instead of silently ignoring them.

Structured safety results use an explicit schema-v2 envelope with `analysis:"safety"`, canonical `SAFE`/`VIOLATED`/`INCONCLUSIVE` status, model-space limits/accounting/cutoff provenance, and dedicated action/state safety trace evidence. Product accounting is absent rather than fabricated and fairness sets are empty. Existing multi-response jobs retain the historical schema-v1 field order, evidence shape, fairness semantics, staged/product accounting, and exit behavior; explicit `analysis "multi-response"` does not inject an `analysis` field into their v1 JSON.

The M57 raw suite and M58 expectation runner remain family-agnostic at the orchestration layer: every job is dispatched through the canonical heterogeneous job runner, complete per-job envelopes are preserved unchanged, raw aggregate precedence remains `error > violated > inconclusive > satisfied`, and expectation matching compares only the normalized observed outcome. Mixed suites therefore carry schema-v1 temporal and schema-v2 safety results side by side without rewriting either family.

Executable evidence covers historical manifest/parser compatibility, explicit family round trips, fail-closed unknown/duplicate analysis names, safe/violated/inconclusive safety jobs against direct M24 results, initial-state violations before zero-transition cutoffs, incompatible directive rejection, schema-v1 multi-response compatibility, mixed-family raw suites, mixed-family matched/mismatched expectation suites, deterministic repeated JSON, and built-binary differential preservation across `fvlab temporal job ... --format json` and `fvlab-suite`.

The implementation candidate `e6a3a31eb121d8993b847485d7635e21992f7b92` passed CI #492: rustfmt, all-target build, Clippy with `-D warnings`, the complete test suite, and every historical CLI regression gate. The same exact candidate passed Bounded state-property CLI workflow #342. This status update itself still requires exact-head CI before merge.

M59 adds no new safety semantics, fairness semantics, fairness-by-default behavior, witness-text golden matching, shell execution, wall-clock proof bounds, generic plugin/RPC subsystem, or performance/security claim.

## Next frontier — Milestone 60: reproducible exact-state property jobs

M59 proves that the job/suite/regression layer can dispatch more than one verification family without erasing family-specific semantics. The highest-value next capability is to admit the already-sealed M20/M24 exact-state property frontend into the same reproducible infrastructure rather than adding another alias for the existing job command or inventing a generic plugin abstraction.

Milestone 60 should add one explicit exact-state analysis family whose property file uses the existing deterministic `reachable("state")` / `all-eventually("state")` syntax and whose execution delegates directly to the bounded exact-state authority.

Acceptance criteria:

- extend the typed job-family discriminator with an exact-state family while preserving every M55–M59 manifest and result unchanged;
- parse property files through the sealed M20 exact-state parser and execute through the M24 bounded exact-state backend, with no second graph traversal or duplicate reachability/eventuality semantics;
- preserve proof-honest model state/transition/depth budgets, exact cutoff reasons/accounting, shortest positive reachability witnesses, and finite/lasso universal-eventuality counterexamples;
- define a versioned family-specific machine envelope only where required, without stuffing temporal pending bits or fairness/product fields into exact-state evidence;
- reject temporal-only fairness and product-space limits for exact-state jobs before model/property execution;
- prove mixed temporal + safety + exact-state raw suites and expectation suites preserve deterministic ordering, aggregate precedence, complete nested envelopes, and direct built-binary equivalence;
- add malformed-property, unreachable reachability, violated eventuality, bounded inconclusive, and conclusive-before-cutoff regressions;
- add no new temporal logic, fairness-by-default behavior, shell execution, wall-clock proof bound, generic plugin/RPC subsystem, or performance/security claim.

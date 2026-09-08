# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 60 — reproducible exact-state verification jobs

**Status: integration candidate complete.**

Milestone 60 extends the M59 heterogeneous verification-job dispatcher with a third explicit family, `analysis "exact-state"`, backed directly by the sealed M20/M24 exact-state frontend rather than a new traversal. Historical manifests without an `analysis` directive remain multi-response jobs, explicit `analysis "multi-response"` keeps schema-v1 behavior, and M59 safety jobs retain their schema-v2 contract.

Exact-state job property files use the existing deterministic `reachable("state")` / `all-eventually("state")` syntax. Models are loaded through the M19 declarative compatibility path, properties are parsed by the M20 exact-state parser, and execution delegates to M24 bounded exact-state verification. Model state/transition/depth budgets therefore preserve the existing proof-honest semantics: a positive reachability witness may conclude before a later cutoff, absence/satisfaction claims require enough exploration to prove them, and a blocking cutoff remains `INCONCLUSIVE` instead of being promoted to proof.

Exact-state jobs reject temporal-only weak/strong fairness directives and product-space limits before model or property files are read. Violations retain state-property exit 11, incomplete proofs use exit 3, malformed/configuration errors use exit 2, and satisfied properties use exit 0.

Structured exact-state results use heterogeneous schema v2 with `analysis:"exact-state"`, canonical `SATISFIED` / `VIOLATED` / `INCONCLUSIVE` status, model-space limits/accounting/cutoff provenance, and dedicated action/state evidence for positive reachability witnesses, finite eventuality counterexamples, and lasso eventuality counterexamples. Product accounting remains absent rather than fabricated, fairness sets remain empty, and exact-state evidence contains no temporal pending bits.

The family-agnostic M57/M58 suite and expectation infrastructure now carries three verification families without reinterpretation. Raw temporal + safety + exact-state suites preserve each direct job envelope exactly and retain aggregate precedence. Expectation suites compare normalized outcomes only, including matched and mismatched exact-state results. Built `fvlab` and `fvlab-suite` paths are differentially checked so nested suite envelopes remain byte-for-byte the same canonical job JSON produced by direct execution.

Executable evidence includes exact-state family parser/canonical round trips; direct M24 differential accounting and shortest reachability witness preservation; unreachable reachability without fabricated evidence; model-cutoff `INCONCLUSIVE` provenance; conclusive initial-state witnesses before zero-transition cutoffs; finite and lasso eventuality evidence preservation; fail-closed temporal-only configuration validation before file reads; native exit-code and machine-JSON checks through the built binary; three-family raw-suite ordering/precedence; expectation match/mismatch behavior; deterministic repeated suite JSON; and built-binary envelope preservation across all three families.

The implementation candidate `c00cf6a54927e229af3139f7a86652ebebd48638` passed CI #502: rustfmt, all-target build, Clippy with `-D warnings`, the complete test suite, and every historical CLI regression gate. The same exact candidate passed Bounded state-property CLI workflow #352. This status update itself still requires exact-head CI before merge.

M60 adds no new reachability/eventuality semantics, temporal logic, fairness behavior, fairness-by-default behavior, shell execution, wall-clock proof bound, generic plugin/RPC subsystem, or performance/security claim.

## Next frontier — Milestone 61: reproducible Boolean proposition property jobs

M60 closes the exact-state reproducibility gap. The next highest-value external verification family is the already-sealed M22/M24 Boolean proposition reachability/eventuality stack, because it can express semantic state classes and Boolean combinations without inventing another model checker or widening the temporal logic surface.

Milestone 61 should add one explicit proposition-expression job family whose property document selects `reachable` or `all-eventually` and carries an M22 Boolean proposition expression. Parsing must remain an ingestion layer: proposition expressions are resolved against the M21 declarative metadata and execution delegates directly to the existing bounded proposition-expression authority.

Acceptance criteria:

- add an explicit Boolean proposition property job family while preserving every M55–M60 manifest, schema, exit code, and evidence shape unchanged;
- define one deterministic, minimal property-file syntax that selects `reachable` or `all-eventually` and reuses the sealed M22 Boolean expression parser instead of duplicating expression semantics;
- execute through M24 bounded proposition-expression verification with exact model state/transition/depth cutoff reasons and no second graph traversal;
- preserve shortest positive reachability witnesses, unreachable-without-fabricated-evidence behavior, and finite/lasso universal-eventuality counterexamples;
- fail closed on unknown proposition references, malformed expressions, temporal fairness directives, and product-space limits before unsupported assumptions can be ignored;
- define family-specific machine evidence over action/state traces only, with no fabricated product accounting or temporal pending bits;
- prove four-family temporal + safety + exact-state + Boolean-proposition raw suites and expectation suites preserve deterministic ordering, aggregate precedence, complete nested envelopes, and direct built-binary equivalence;
- add generated or differential validation against direct M22/M24 results across representative Boolean expressions and resource cutoffs rather than duplicating backend expected values;
- add no arithmetic/state-field expression language, new liveness semantics, fairness-by-default behavior, shell execution, wall-clock proof bound, generic plugin/RPC subsystem, or performance/security claim.

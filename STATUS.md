# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 61 — reproducible Boolean proposition-expression verification jobs

**Status: integration candidate complete.**

Milestone 61 extends the M59/M60 heterogeneous verification-job dispatcher with a fourth explicit family, `analysis "proposition-expression"`, backed directly by the sealed M21/M22/M24 proposition-expression stack rather than a new traversal. Historical manifests without an `analysis` directive remain multi-response jobs, explicit `analysis "multi-response"` keeps schema-v1 behavior, and M59 safety plus M60 exact-state jobs retain their schema-v2 contracts unchanged.

Proposition-expression property files use one deliberately small deterministic surface: `reachable <expression>` or `all-eventually <expression>`, where `<expression>` is parsed by the existing M22 Boolean proposition grammar. Declarative models are loaded with M21 proposition metadata, complete expressions are resolved before execution, and verification delegates to `check_proposition_expression_property_with_limits`. No Boolean semantics, state-space traversal, reachability semantics, or universal-eventuality semantics are reimplemented in the job layer.

Model state/transition/depth budgets preserve M24 proof honesty. A positive reachability witness may conclude before a later cutoff. An unreachable reachability claim or satisfied universal-eventuality claim requires enough exploration to prove it. A real finite terminal or target-avoiding lasso remains a conclusive violation when already justified. A blocking resource cutoff remains `INCONCLUSIVE` with the exact model-stage state/transition/depth reason instead of being promoted to proof.

Proposition-expression jobs reject temporal-only weak/strong fairness directives and product-space limits before model or property files are read. Unknown proposition references and malformed property modes/Boolean syntax fail closed. Satisfied properties use exit 0, state-property violations retain exit 11, incomplete proofs use exit 3, and malformed/configuration errors use exit 2.

Structured results use heterogeneous schema v2 with `analysis:"proposition-expression"`, canonical `SATISFIED` / `VIOLATED` / `INCONCLUSIVE` status, model-space limits/accounting/cutoff provenance, and action/state evidence only. Positive reachability preserves the deterministic shortest witness. Universal-eventuality violations preserve finite or lasso evidence. Unreachable existential queries do not fabricate counterexamples. Product accounting remains absent, fairness sets remain empty, and no temporal pending bits are manufactured.

The M57/M58 suite and expectation infrastructure now carries four verification families without reinterpretation. Raw temporal + safety + exact-state + proposition-expression suites preserve every direct nested job envelope in declaration order and retain aggregate outcome precedence. Expectation suites compare normalized outcomes only, including matched and mismatched proposition-expression jobs. Built `fvlab` and `fvlab-suite` paths are differentially checked so nested suite results contain the same canonical direct job JSON.

Executable evidence includes parser/canonical round trips; proof-honest cutoff behavior; unknown-reference and malformed-input rejection; shortest reachability witness preservation; finite and lasso universal-eventuality evidence; built-binary native exit codes and schema-v2 JSON; a **40-case** job-vs-direct-backend matrix over both property modes, representative Boolean expressions, and model limit profiles; plus four-family raw-suite and expectation-suite direct-envelope differential coverage. The complete historical CI suite remains unchanged as the compatibility gate for M1–M60 behavior.

The implementation candidate `38d9c10214a771506617b981c18d946193b4a89a` passed CI #511: rustfmt, all-target build, Clippy with `-D warnings`, the complete test suite, and every historical CLI regression gate. The same exact candidate passed Bounded state-property CLI workflow #361. This status update itself still requires exact-head CI before merge.

M61 adds no arithmetic/state-field expression language, new liveness semantics, fairness behavior, fairness-by-default behavior, shell execution, wall-clock proof bound, generic plugin/RPC subsystem, or performance/security claim.

## Next frontier — Milestone 62: reproducible action-temporal verification jobs

M61 closes the state-predicate reproducibility gap. The next highest-value external verification family is the already-sealed M17/M18/M27/M47 action-temporal frontend, because reproducible jobs currently cover historical multi-response properties and state-property families but cannot explicitly run the existing textual `response(...)` / `infinitely-often(...)` action-temporal language as a heterogeneous job family.

Milestone 62 should add an explicit `analysis "action-temporal"` family whose property file is parsed by the existing M18 textual temporal parser and whose execution delegates to the sealed typed/textual/declarative temporal authorities. It must support the temporal features already available in those authorities rather than creating a second semantics layer.

Acceptance criteria:

- add explicit action-temporal verification jobs while preserving every M55–M61 manifest, schema, exit code, aggregate-suite rule, and evidence shape unchanged;
- reuse the existing textual `response("trigger","response")` and `infinitely-often("action"[, ...])` parser/canonical form instead of introducing another property grammar;
- route response and recurring-action forms through the existing canonical temporal backends, preserving backend identity, deterministic finite/lasso evidence, and no-fairness compatibility;
- carry model and product state/transition/depth budgets through the already-sealed staged temporal APIs with exact model-before-product cutoff provenance and no fabricated proof from partial prefixes;
- carry the existing explicit weak/strong fairness profile only where the canonical action-temporal API supports it, preserving overlap canonicalization and fairness-on-infinite-executions-only semantics;
- define deterministic schema-v2 machine evidence for frontend-level temporal traces without exposing internal monitor/Büchi control states or changing historical schema-v1 multi-response envelopes;
- fail closed on unsupported/malformed fairness, property syntax, budgets, model files, and references before assumptions can be silently ignored;
- prove five-family raw and expectation suites preserve declaration order, aggregate precedence, complete nested envelopes, deterministic repetition, and direct built-binary equivalence;
- add differential validation against direct action-temporal execution across both supported textual forms, fairness profiles, and bounded/staged cutoff profiles rather than duplicating backend expected values;
- add no new temporal operators, arbitrary LTL/CTL compilation, fairness-by-default behavior, shell execution, wall-clock proof bound, generic plugin/RPC subsystem, or performance/security claim.

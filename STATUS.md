# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 64 — proof-honest bounded recurrence / cycle discovery

**Status: sealed in `main` at `7a384ac9a66c31c767f1124fd08319645f79d01b`.**

Milestone 64 extends recurrence analysis with deterministic model-space state, transition, and depth budgets while preserving the sealed exhaustive `analyze_recurrence` behavior.

`analyze_recurrence_with_limits` reuses `capture_reachable_graph_with_limits` and the existing iterative Tarjan/shortest-path substrate. Any closed cycle entirely contained in retained explored edges is a real full-model cycle, so it is conclusively reported as `CycleFound` even when exploration later becomes incomplete. By contrast, absence of a cycle proves `Acyclic` only when reachable-graph capture completes; an incomplete acyclic prefix is explicitly `INCONCLUSIVE`.

Bounded results expose exact discovery/check/transition/depth accounting and deterministic stem-plus-cycle evidence. A full SCC partition is exposed only when capture completed. Incomplete prefixes never masquerade as the full-model partition. The independent **4,608-case** oracle covers all 512 directed three-state graphs × 9 deterministic limit profiles, while the sealed 512-graph SCC oracle and 50,000-node iterative deep-chain/deep-cycle regressions remain migration gates.

Post-merge exact-main CI #552 and Bounded state-property CLI workflow #402 both succeeded.

## Milestone 65 — declarative bounded recurrence reporting and file CLI

**Status: integration candidate complete; exact-head verified before closure metadata.**

Milestone 65 exposes the M64 proof boundary to external declarative finite models without converting structural recurrence into a pass/fail verification property.

The bounded recurrence result now preserves `cutoff_reason` even when a retained real cycle makes the structural outcome conclusively `CycleFound`. This closes a provenance gap: the report can truthfully distinguish “cycle proved and exploration complete” from “cycle proved before a later resource cutoff.” The proof status itself is unchanged.

`render_bounded_recurrence_report` renders `CYCLE_FOUND`, `ACYCLIC`, or `INCONCLUSIVE`, exact model accounting, cutoff provenance, and deterministic cycle evidence. A complete SCC partition is rendered only when exploration completed; otherwise the report explicitly says the partition is unavailable rather than presenting a prefix partition as exhaustive.

`fvlab scc file <path> [--max-states N] [--max-transitions N] [--max-depth N]` loads the existing declarative `.fvl` graph grammar and delegates directly to `analyze_recurrence_with_limits`. Both conclusive structural outcomes exit 0, incomplete exploration exits 3, and malformed file/model/options exit 2. Historical `scc counter` and `scc traffic-light` continue to use the sealed unbounded renderer and retain their existing surface.

Executable evidence includes:
- the unchanged M64 **4,608-case** bounded recurrence oracle, now also checking exact cutoff provenance;
- six built-binary regressions covering complete acyclic, complete cyclic, cycle-before-later-cutoff, cutoff-without-cycle, malformed model/options, and historical hard-coded SCC compatibility;
- direct backend/report versus external-file CLI output equality on the new paths;
- exact candidate `87b0ea56c97ca911208b52cc2df3be601b0db99c` passed CI #555 (format, build, Clippy with `-D warnings`, full tests, and historical CLI gates) and Bounded state-property CLI workflow #405;
- CI #555 executed `tests/bounded_recurrence.rs` 6/6 including the generated oracle and `tests/bounded_recurrence_cli.rs` 6/6.

M65 introduces no verification-job family, fairness interpretation, symbolic engine, second recurrence semantics, wall-clock proof bound, or performance claim.

## Next frontier — Milestone 66: neutral structural-analysis job protocol

The current reproducible verification-job stack assumes every analysis has a required property file and normalizes outcomes to `satisfied` / `violated`. That shape is appropriate for property verification but is semantically wrong for recurrence: `CycleFound` and `Acyclic` are both neutral structural analysis results unless a separate user expectation turns one into a regression condition.

Milestone 66 should add a reproducible **structural-analysis** protocol instead of forcing recurrence into the property-verification envelope.

Acceptance criteria:

- define an explicit recurrence/structural job family whose manifest does not require a fake property file;
- preserve model-space state/transition/depth budgets and reject temporal-only fairness/product-limit directives fail-closed;
- define a versioned structural result envelope with neutral `CYCLE_FOUND`, `ACYCLIC`, `INCONCLUSIVE`, and `ERROR` outcomes rather than reusing `VerificationJobOutcome::{Satisfied,Violated}`;
- preserve exact model accounting, cutoff provenance, complete-partition availability, and deterministic stem-plus-cycle evidence;
- route execution through the sealed `analyze_recurrence_with_limits` authority; do not add another SCC or graph traversal implementation;
- provide canonical manifest parse/render round trips and path resolution relative to the manifest;
- add built-binary JSON job integration for complete acyclic/cyclic, conclusive-cycle-before-cutoff, incomplete acyclic prefix, malformed input, and incompatible directives;
- keep the existing property-verification job schema and six established verification families byte/behavior compatible;
- defer suite expectation semantics until the neutral structural result protocol is stable; a later milestone may define explicit expectations such as “expect acyclic” without redefining raw cycle discovery itself;
- add no fairness interpretation, symbolic engine, wall-clock proof bound, or performance claim.

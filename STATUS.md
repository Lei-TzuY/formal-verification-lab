# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 63 — declarative bounded deadlock policies and reproducible jobs

**Status: sealed in `main` at `eb8c30aa76f58188aac3ec7b5c85c55ac3b23a76`.**

Milestone 63 closes the external deadlock-policy gap as one backend-to-job vertical slice. The canonical deadlock backend now exposes proof-honest state/transition/depth limits; a genuine unexpected terminal discovered before a later cutoff remains conclusively `DEADLOCK_FOUND`, while `DEADLOCK_FREE` requires exhaustive completion. Terminal detection remains after complete successor-vector evaluation, so a cutoff cannot fabricate a terminal.

The declarative frontend reuses the existing Boolean proposition-expression semantics and one shared pre-resolved evaluator. The external `fvlab-deadlock` binary and explicit `analysis "deadlock"` verification jobs route through the same authority. Temporal-only fairness and product-limit settings fail closed. Schema-v2 results preserve model-only accounting and shortest state/action evidence without inventing product state.

The bounded backend is independently checked by **36,864** generated graph/policy/limit cases, and raw/expectation suites cover six analysis families. Post-merge exact-main CI #545 and Bounded state-property CLI workflow #395 both succeeded.

## Milestone 64 — proof-honest bounded recurrence / cycle discovery

**Status: integration candidate complete; exact-head verified before closure metadata.**

Milestone 64 extends recurrence analysis with deterministic model-space state, transition, and depth budgets while preserving the sealed exhaustive `analyze_recurrence` behavior.

`analyze_recurrence_with_limits` reuses `capture_reachable_graph_with_limits` and the existing iterative Tarjan/shortest-path substrate. Any closed cycle entirely contained in retained explored edges is a real full-model cycle, so it is conclusively reported as `CycleFound` even when exploration later becomes incomplete. By contrast, absence of a cycle proves `Acyclic` only when reachable-graph capture completes; an incomplete acyclic prefix is explicitly `INCONCLUSIVE`.

Bounded results expose exact discovery/check/transition/depth accounting and an optional deterministic stem-plus-cycle witness. A full SCC partition is exposed only when capture completed. Incomplete prefixes never masquerade as the full-model partition. When a cycle is found in an incomplete prefix, the witness remains a real path and closed cycle; its component index is explicitly only the retained-prefix SCC ordering.

Executable evidence includes:
- focused retained self-loop and two-state cycle cases that remain conclusive before a later cutoff;
- cutoff-without-cycle cases that remain `INCONCLUSIVE`;
- exact-bound acyclicity proofs;
- exact equality between unbounded-limit M64 results and the sealed M8 recurrence structure/witness on all 512 generated three-state graphs;
- an independent **4,608-case** oracle over all 512 directed three-state graphs × 9 deterministic limit profiles, independently reproducing canonical cutoff precedence, retained-edge materialization, mutual reachability, cycle classification, accounting, shortest stem distance, real-edge witness validity, and determinism;
- the existing 512-graph SCC oracle and 50,000-node iterative deep-chain/deep-cycle regressions remain unchanged migration gates.

Candidate `9655dea89e248742b7a77895204a92a95854f116` passed CI #550 (format, build, Clippy with `-D warnings`, full tests, and historical CLI gates) and Bounded state-property CLI workflow #400 before this closure-only status update.

M64 adds no partial-SCC proof claim, fairness interpretation, symbolic-state engine, second traversal engine, wall-clock proof bound, or performance claim.

## Next frontier — Milestone 65: declarative bounded recurrence reporting and file CLI

M64 establishes the core proof boundary. The next useful layer is to expose that capability to external declarative finite models without converting structural recurrence into a pass/fail verification property.

Acceptance criteria:

- add a stable bounded recurrence report that renders `CYCLE_FOUND`, `ACYCLIC`, or `INCONCLUSIVE`, exact model accounting, cutoff reason, and deterministic cycle evidence when present;
- render the complete SCC partition only when the bounded result actually contains a complete partition; never print a partial prefix partition as if exhaustive;
- add a declarative file path under the recurrence/SCC CLI surface, accepting the existing `.fvl` model grammar plus `--max-states`, `--max-transitions`, and `--max-depth`;
- keep structural semantics neutral: both conclusive `CYCLE_FOUND` and `ACYCLIC` are successful analyses (exit 0), incomplete exploration exits 3, malformed file/model/options exit 2;
- preserve the existing hard-coded `scc counter` and `scc traffic-light` behavior unchanged;
- add built-binary integration covering complete acyclic, complete cyclic, cycle-before-later-cutoff, cutoff-without-cycle, malformed model, and malformed/duplicate limit options;
- differentially compare declarative-file results with direct `analyze_recurrence_with_limits` calls so the CLI/report layer adds no second recurrence semantics;
- do not introduce a heterogeneous verification-job family until a separate architecture decision defines how structural cycle information should map (or not map) to pass/fail expectations;
- add no fairness interpretation, symbolic-state engine, second traversal engine, wall-clock proof bound, or performance claim.

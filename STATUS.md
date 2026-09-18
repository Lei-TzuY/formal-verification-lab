# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 62 — reproducible action-temporal verification jobs

**Status: sealed in `main` at `09907e6a58c23407363dca76568374bbd6a881ac`.**

Milestone 62 extends the heterogeneous verification-job dispatcher with a fifth explicit family, `analysis "action-temporal"`, without changing the historical implicit multi-response default or the sealed safety, exact-state, and proposition-expression job contracts.

Action-temporal property files reuse the existing M18 textual grammar exactly: `response("trigger","response")` and `infinitely-often("action"[, ...])`. The job layer does not implement temporal semantics. It parses the existing specification, canonicalizes it through the existing frontend, and delegates execution to the sealed action-temporal authority.

Model/product budgets, exact-action fairness profiles, schema-v2 normalized evidence, backend identity, cutoff provenance, five-family raw/expectation suites, and built-binary equivalence remain covered by the historical M62 tests. M62 adds no new temporal operator, traversal engine, fairness-by-default behavior, shell execution, wall-clock proof bound, or performance/security claim.

## Milestone 63 — declarative bounded deadlock policies and reproducible jobs

**Status: integration candidate complete; exact-head verified.**

Milestone 63 closes the external deadlock-policy gap as one backend-to-job vertical slice rather than a wrapper-only family.

The canonical deadlock backend now exposes `check_deadlock_with_limits` over the existing deterministic `search_with_probes` BFS substrate. State/transition/depth cutoffs return explicit `INCONCLUSIVE`; a genuine unexpected terminal discovered before a later cutoff remains conclusively `DEADLOCK_FOUND`; `DEADLOCK_FREE` requires exhaustive completion. Terminal detection remains in the `after_successors` probe, so partial exploration cannot fabricate a terminal.

`DeclarativeDeadlockSpec` reuses the existing M22 Boolean proposition-expression grammar and one shared pre-resolved evaluator. Every proposition reference is resolved before backend exploration. No second Boolean semantics or graph traversal was introduced.

The external `fvlab-deadlock` binary accepts a declarative model file, a legitimate-terminal proposition expression, and model-space `--max-states`, `--max-transitions`, and `--max-depth` limits. Deadlock-free exits 0, a real unexpected terminal exits 5, incomplete exploration exits 3, and malformed/model/reference/configuration errors exit 2.

The heterogeneous job dispatcher now accepts explicit `analysis "deadlock"` jobs backed by the same declarative frontend. Temporal-only fairness assumptions and product limits are rejected before deadlock execution. Schema-v2 results carry model-only accounting and cutoff provenance plus action/state shortest-witness evidence; product accounting, pending vectors, and temporal control state remain absent.

Raw and expectation suites now cover six analysis families while preserving declaration order, aggregate precedence, complete nested envelopes, deterministic repetition, and direct built-binary equivalence. The bounded backend is independently checked by a **36,864-case** oracle over all 512 directed three-state graphs × 8 legitimate-terminal policies × 9 deterministic limit profiles, including initial terminals, exact-bound completion, cutoff-before-witness, witness-before-later-cutoff, and no-false-terminal cases.

Exact candidate `96e4cae04df59c6e8b120a750835d1c258ef5ea6` passed CI #543 and Bounded state-property CLI workflow #393. The branch is based on `main=09907e6a58c23407363dca76568374bbd6a881ac`, behind 0, mergeable, with no reviews or unresolved review threads at closure audit time.

M63 adds no fairness interpretation for finite deadlocks, symbolic-state engine, arithmetic state-expression language, second traversal engine, wall-clock proof bound, or performance/security claim.

## Next frontier — Milestone 64: proof-honest bounded recurrence / cycle discovery

M63 seals the remaining externally configurable finite-terminal policy path. The next core architectural gap is recurrence: M8 has a trusted deterministic SCC/cycle analysis over a completely captured reachable graph, but it is exhaustive-only. A resource cutoff cannot currently be represented without either abandoning the analysis or risking an unsound “acyclic” conclusion from a partial prefix.

Milestone 64 should add bounded recurrence as a core semantic capability before introducing another declarative/job wrapper.

Acceptance criteria:

- add deterministic state/transition/depth budgets to recurrence discovery without changing the sealed unbounded `analyze_recurrence` result;
- treat a real closed cycle entirely contained in retained explored edges as a conclusive positive witness even when a later cutoff would otherwise occur;
- report conclusive acyclicity only after complete reachable-graph exploration; a cutoff with no justified cycle must be `INCONCLUSIVE`, never “acyclic”;
- preserve canonical model-limit precedence and exact model accounting, and distinguish retained edges from unknown successors so a cutoff cannot fabricate a terminal, SCC boundary, or cycle;
- preserve deterministic shortest global stem to the reported cycle entry and a real-edge closed cycle witness; do not claim the returned cycle is globally shortest unless proved;
- keep exhaustive SCC partition reporting on the sealed unbounded API only unless the bounded exploration completed; do not present partial SCCs as the full-model partition;
- validate semantics with an independent generated small-graph × limit-profile oracle covering cutoff-before-cycle, cycle-before-later-cutoff, exact-bound completion, self-loops, multi-node cycles, and acyclic prefixes;
- retain the existing 512-graph SCC oracle and 50,000-node iterative deep-graph regressions unchanged as migration gates;
- add no fairness interpretation, symbolic-state engine, second traversal engine, wall-clock proof bound, or performance claim.

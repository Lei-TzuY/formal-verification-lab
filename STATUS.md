# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; this file is the canonical short-form phase marker.

## Milestone 64 — proof-honest bounded recurrence

**Status: sealed in `main` at `7a384ac9a66c31c767f1124fd08319645f79d01b`.**

M64 adds deterministic model-space state/transition/depth budgets to recurrence while preserving the exhaustive `analyze_recurrence` contract. Retained real cycles are conclusive even if a later cutoff occurs; an acyclic conclusion requires complete reachable-graph capture. Incomplete prefixes never masquerade as the full SCC partition. The independent 4,608-case bounded recurrence oracle plus the historical SCC/deep-graph regressions remain gates.

## Milestone 65 — declarative bounded recurrence CLI

**Status: sealed in `main` at `aec42f5a77c89534e44f3401b2dd8066d5025102`.**

M65 exposes the M64 proof boundary to declarative model files through `fvlab scc file <path> [limits]`, preserving cutoff provenance even after conclusive cycle discovery and withholding complete-partition claims when exploration is incomplete. Conclusive `CYCLE_FOUND` / `ACYCLIC` exit 0, incomplete exploration exits 3, and malformed input exits 2.

## Milestone 66 — neutral structural-analysis job protocol

**Status: sealed in `main` at `78f5e21eb113b2f57de9dc021934c1bbb4117f8b`.**

M66 adds a reproducible structural job protocol without forcing recurrence into the property-verification `satisfied` / `violated` envelope. Manifest-relative model loading delegates directly to the sealed bounded recurrence authority, versioned JSON preserves neutral `cycle_found`, `acyclic`, `inconclusive`, and `error` outcomes, and incomplete prefixes never expose a fake exhaustive SCC partition.

## Milestone 67 — deterministic structural-analysis suites and expectations

**Status: sealed in `main` at `58770b29ee4be73dad59c9a880034abd9ee2754c`.**

M67 adds deterministic multi-job orchestration over the sealed M66 structural-job protocol. Raw suites preserve neutral `cycle_found`, `acyclic`, `inconclusive`, and `error` envelopes without property-style reinterpretation; a separate explicit expectation mode supplies regression meaning. Exact candidate `5ef77ebe6cdccf045ff4dcaee458da4bb84d8ee0` passed CI #575 and Bounded state-property CLI #425 before squash integration.

## Milestone 68 — validated partial-order reduction foundation

**Status: integration candidate complete on PR #69.**

M68 promotes the old M5 safety-reduction experiment from raw-declaration-only auditing to a validation-first proof boundary while preserving the historical audit API.

Implemented contract:

- raw `IndependenceRelation` remains non-authoritative; `audit_sleep_set_reduction` still differentially compares raw-declaration reduction with canonical exhaustive safety;
- `validate_independence` captures the complete canonical reachable graph once and binds exact action-pair claims to that snapshot plus its invariant observations;
- validation fails closed on configured same-label nondeterminism, enabledness changes under the peer action, non-commuting diamonds, and intermediate invariant-observation changes;
- `ValidatedIndependenceRelation` has private evidence fields and the standalone `check_validated_sleep_set_reduction` accepts only that evidence, executing over the exact certified snapshot rather than a caller-supplied model;
- raw and validated reducers preserve distinct `(state, sleep-set)` contexts, so revisiting the same model state under a different sleep set is not collapsed by one global state-only visited entry;
- reduction witnesses are deterministic DFS evidence, not advertised as shortest BFS witnesses;
- generated differential coverage enumerates all 4,096 deterministic two-action graphs over three states across all eight safety masks (32,768 model/property combinations); every relation accepted by the validator is compared with canonical exhaustive safety;
- focused regressions cover invalid declarations, enabledness interference, commuting/invariant-observation failures, same-label ambiguity, sleep-context revisitation, deterministic witnesses, and the commuting-counter baseline.

Candidate `97b253d9b8f2f92bf9e8bd02e194d99c0154fe36` passed CI #580 (format, all-target build, Clippy with `-D warnings`, full tests including the generated differential, and all historical CLI gates) and Bounded state-property CLI #430. Closure metadata changes must pass the same exact-head gates before merge.

M68 is a proof-safety foundation, not a performance claim: validation itself exhaustively captures the reachable graph. It makes no claim about liveness POR, arbitrary independence theories, symbolic reduction, or CI timing as a benchmark.

## Next frontier — Milestone 69: CTL branching-time fixpoint kernel

Promote from specialized temporal properties to a general branching-time state-formula authority over the shared complete reachable graph.

Acceptance criteria:

- typed CTL formulas for Boolean composition plus `EX`, `AX`, `EF`, `AF`, `EG`, `AG`, `E[U]`, and `A[U]`;
- one explicit terminal-state policy applied consistently to all operators and tests;
- deterministic least/greatest-fixpoint evaluation over one captured `ReachableGraph`, with memoized subformula state sets rather than repeated model traversal;
- deterministic existential evidence and universal counterevidence where sound, without unearned shortest-witness claims;
- independent generated small-graph/proposition oracle coverage plus terminal/cycle/nesting/duality regressions;
- typed semantic kernel first; parser/CLI/reporting only after the authority layer is sealed;
- preserve every historical verification, fairness, structural, suite, and validated-reduction gate.

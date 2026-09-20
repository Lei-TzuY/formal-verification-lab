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

**Status: implementation in progress on `feature/milestone-68-validated-reduction`.**

Current vertical slice:

- raw `IndependenceRelation` remains non-authoritative and the historical exhaustive differential audit remains available;
- `validate_independence` exhaustively binds exact action-pair claims to one canonical reachable snapshot and its invariant observations;
- validation fails closed on configured same-label nondeterminism, action enabledness changes, non-commuting diamonds, and intermediate invariant-observation changes;
- `ValidatedIndependenceRelation` has private evidence fields and is the only input accepted by the new standalone reduced safety path;
- validated reduction executes over the exact certified snapshot rather than accepting a certificate alongside an arbitrary model;
- raw and validated reducers distinguish the same model state reached under different sleep sets, preventing global state-only visitation from collapsing reduction contexts;
- focused regressions cover invalid declarations, observation changes, context revisitation, deterministic witnesses, and the commuting-counter baseline;
- generated differential coverage enumerates all 4,096 deterministic two-action graphs over three states across all eight safety masks, accepting only relations that pass validation and comparing every accepted reduced result with canonical exhaustive safety.

Acceptance before closure:

- exact-head format/build/Clippy/full tests and all historical CLI gates must pass;
- generated differential must converge without weakening validation or expected safety semantics;
- README/trust-boundary documentation must describe the new validated path without claiming performance from CI timing;
- branch must remain based on current `main`, mergeable, and free of review blockers.

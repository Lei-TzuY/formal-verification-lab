# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; this file is the canonical short-form phase marker.

## Milestone 64 — proof-honest bounded recurrence

**Status: sealed in `main` at `7a384ac9a66c31c767f1124fd08319645f79d01b`.**

M64 adds deterministic model-space state/transition/depth budgets to recurrence while preserving the exhaustive `analyze_recurrence` contract. Retained real cycles are conclusive even if a later cutoff occurs; an acyclic conclusion requires complete reachable-graph capture. Incomplete prefixes never masquerade as the full SCC partition. The independent 4,608-case bounded recurrence oracle plus the historical SCC/deep-graph regressions remain gates.

## Milestone 65 — declarative bounded recurrence CLI

**Status: sealed in `main` at `aec42f5a77c89534e44f3401b2dd8066d5025102`.**

M65 exposes the M64 proof boundary to declarative model files through `fvlab scc file <path> [limits]`, preserving cutoff provenance even after conclusive cycle discovery and withholding complete-partition claims when exploration is incomplete. Conclusive `CYCLE_FOUND` / `ACYCLIC` exit 0, incomplete exploration exits 3, and malformed input exits 2.

## Milestone 66 — neutral structural-analysis job protocol

**Status: integration candidate complete on PR #67.**

M66 adds a reproducible structural job protocol without forcing recurrence into the property-verification `satisfied` / `violated` envelope.

Implemented contract:

- manifest grammar: `analysis "recurrence"`, required `model`, optional model-space `max-states`, `max-transitions`, and `max-depth`;
- property, fairness, and product-limit directives fail closed;
- manifest-relative model resolution and direct delegation to the sealed `analyze_recurrence_with_limits` authority;
- versioned neutral JSON outcomes `cycle_found`, `acyclic`, `inconclusive`, and `error`;
- exact accounting and cutoff provenance, including conclusive retained cycles found before a later cutoff;
- complete SCC partitions only after complete capture; incomplete prefixes expose no fake exhaustive partition;
- deterministic stem-plus-cycle evidence;
- `fvlab scc job <manifest> [--format json]` with exits 0 / 3 / 2 for conclusive structural result / inconclusive / error;
- canonical manifest round trips, schema regressions, external-file execution, malformed-input coverage, and historical integration gates.

Exact pre-closure candidate `a4dbcdad32223718c9983f6e7ac4f616d5f9b773` passed CI #563 (format, build, Clippy with `-D warnings`, full tests, all historical CLI gates) and Bounded state-property CLI #413. Closure metadata changes must pass the same exact-head gates before merge.

M66 adds no second SCC implementation, fairness interpretation, symbolic engine, wall-clock proof bound, or performance claim.

## Next frontier — Milestone 67: deterministic structural-analysis suites and expectations

M66 stabilizes one neutral structural job and its result envelope. The next architectural gap is reproducible multi-job orchestration that preserves structural neutrality while allowing explicit regression expectations as a separate contract.

Acceptance criteria:

- define a dedicated structural-suite manifest with deterministic job order, manifest-relative job paths, and a bounded suite size;
- run each entry through the sealed M66 job runner and preserve its raw neutral envelope;
- keep raw suite execution neutral; do not turn `cycle_found` or `acyclic` into verification pass/fail states;
- add a separate expectation mode with explicit expected structural outcomes and deterministic mismatch reporting;
- define stable aggregate JSON, exit behavior, malformed/missing-job handling, and outcome precedence;
- validate ordered mixed-outcome suites, relative paths, matching/mismatching expectations, raw-vs-CLI equivalence, and fail-closed invalid manifests;
- preserve the existing property-verification suite protocol byte/behavior compatibility and add no new graph traversal, fairness semantics, wall-clock proof bound, or performance claim.

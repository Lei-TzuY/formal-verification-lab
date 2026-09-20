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

**Status: integration candidate complete on PR #68.**

M67 adds deterministic orchestration over the sealed M66 structural-job protocol while preserving structural neutrality.

Implemented contract:

- dedicated structural-suite manifests with declaration-order execution, manifest-relative job paths, duplicate rejection, and a 128-job limit;
- raw suites preserve each complete M66 result envelope and aggregate only operational completion as `complete`, `inconclusive`, or `error`;
- raw execution never maps `cycle_found` / `acyclic` into verification-style pass/fail states;
- a separate expectation mode requires explicit expected structural outcomes and reports deterministic per-job matches/mismatches;
- versioned raw and regression JSON envelopes with deterministic ordering, missing/malformed-job preservation, aggregate precedence, and dedicated mismatch exit 13;
- built `fvlab-structural-suite` binary differential coverage against the library runner plus semantic parser/execution regressions.

Implementation candidate `b04fd6b4d571186a8668cb94bade61785b1e456c` passed CI #573 (format, all-target build, Clippy with `-D warnings`, full tests, and historical CLI gates) and Bounded state-property CLI #423. Closure metadata changes must pass the same exact-head gates before merge.

M67 adds no second recurrence/SCC traversal, fairness interpretation, wall-clock proof bound, or performance claim.

## Next frontier — Milestone 68: validated partial-order reduction foundation

The long-standing M5 sleep-set reduction remains deliberately experimental because caller-supplied action independence is not itself proof evidence and the reduced search is only trusted after exhaustive differential comparison. The next architectural phase is to turn that old experiment into a validation-first reduction substrate without weakening the canonical exhaustive checker.

Acceptance criteria:

- add an explicit reachable-graph independence validator for exact action-label pairs instead of trusting declarations;
- validate conservative local conditions needed by the supported safety use case, including enabledness preservation, commuting diamonds, deterministic handling of ambiguous same-label successors, and invariant-observation preservation; unsupported or ambiguous cases fail closed;
- introduce a validated/certified independence type that cannot be constructed from unchecked declarations and keep the historical audited API behavior-compatible;
- make reduced exploration safe under state revisitation by preserving the sleep-set context required for completeness rather than treating one visited state as equivalent to every sleep context;
- expose a standalone reduced safety path only through validated independence evidence; do not claim reduction correctness for raw declarations;
- add generated differential coverage over small finite graphs/declarations against exhaustive safety, plus focused invalid-declaration, commuting-diamond, invariant-visibility, revisit-context, and deterministic-witness regressions;
- report actual exploration/pruning counts as observations only; make no performance claim from CI timing;
- preserve every existing verification/structural API and all historical CI gates.

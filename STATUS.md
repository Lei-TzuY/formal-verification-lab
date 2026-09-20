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

**Status: sealed in `main` at `1bf1395253bb0f2f0d2cd96b25c03199ef2528dd`.**

M68 promotes the old M5 safety-reduction experiment from raw-declaration-only auditing to a validation-first proof boundary. Raw declarations remain non-authoritative; validated certificates are bound to one complete canonical reachable snapshot plus invariant observations, fail closed on supported independence violations, and preserve distinct `(state, sleep-set)` contexts. Exact candidate `f44e7d56fc919ebee0a1245d82c23cdff35ab4b9` passed CI #582 and Bounded state-property CLI #432 before squash integration.

## Milestone 69 — typed CTL branching-time fixpoint kernel

**Status: sealed in `main` at `39d849bc6327f7d0e91628410b23046aa567dfca`.**

M69 adds the parser-free typed CTL semantic authority: Boolean composition plus `EX`, `AX`, `EF`, `AF`, `EG`, `AG`, `E[U]`, and `A[U]` over one complete canonical reachable graph, explicit terminal self-loop totalization, memoized fixpoint state sets, and deterministic finite/lasso evidence. Exact candidate `a38773c27ff6bcf50078e5567b2a82b54a3ec149` passed CI #588 and Bounded state-property CLI #438 before squash integration.

## Milestone 70 — declarative CTL parser and file frontend

**Status: sealed in `main` at `4b8b3a192fe19bb07f557e65557c6b38f5ebbb6d`.**

M70 exposes the sealed M69 authority through deterministic textual CTL parsing, existing declarative named-proposition binding, stable finite/lasso reporting, and `fvlab ctl file` execution. Unknown atoms fail before backend graph capture. Exact candidate `f474b74498bec2913ab14c291c71d01612ffe74d` passed CI #595 and Bounded state-property CLI #445 before squash integration.

## Milestone 71 — proof-honest bounded CTL semantics

**Status: sealed in `main` at `887b5f5f8eb0bb44995ee3b991d92bf8b013c368`.**

M71 adds conservative lower/upper CTL satisfaction sets over the canonical bounded reachable-graph capture, preserves unknown successor behavior at cut states, only totalizes proven terminals, emits only justified retained evidence, and collapses complete captures back to the sealed M69 authority. Exact candidate `ef6b0ba8412c2949e4155b6067e71768db83cc9c` passed CI #602 and Bounded state-property CLI #452 before squash integration.

## Milestone 72 — bounded declarative CTL file frontend

**Status: sealed in `main` at `aeb96ea16796ccb53480f78ecb656858777b2db1`.**

M72 exposes M71 through the declarative named-proposition frontend and `fvlab ctl file` model-space limits while preserving the no-option complete path. Reports keep lower/upper state-set counts, per-initial three-valued status, justified evidence, and exact cutoff provenance. Exact candidate `1850b7375f066f4cef5faeb80f9dbf64b8a5e0b7` passed CI #607 and Bounded state-property CLI #457 before squash integration.

## Milestone 73 — reproducible CTL verification jobs

**Status: implementation candidate complete on PR #74.**

M73 adds CTL as a first-class family in the existing heterogeneous verification-job protocol without creating a second parser, semantic engine, or job runner.

Implemented contract:

- `analysis "ctl"` is accepted by the existing verification-job manifest grammar while historical manifests without an analysis directive remain multi-response jobs;
- manifest-relative `model` and `property` files carry the declarative model and textual CTL formula;
- jobs with no model limits delegate to the sealed M70 complete frontend; configured `max-model-states`, `max-model-transitions`, or `max-model-depth` delegate to M72/M71 bounded semantics;
- CTL jobs reject weak/strong fairness and all `max-product-*` directives before reading referenced input files;
- schema-v3 CTL result envelopes preserve canonical `satisfied`, `violated`, `inconclusive`, and `error` outcomes, model-stage cutoff provenance, model accounting, and the canonical CTL property text;
- CTL-specific machine-readable details preserve retained/lower/upper state-set counts, whether all initial states were retained, per-initial `true`/`false`/`unknown`, and normalized finite/lasso evidence;
- synthetic terminal self-loops remain explicitly distinguishable from real model actions in JSON;
- non-CTL envelopes retain their historical JSON shape because the CTL extension is emitted only for CTL results;
- the existing `fvlab temporal job <manifest> --format json` route remains the sole built-binary job runner;
- regressions cover analysis round-trip, complete direct-vs-job satisfaction/violation, all three model cutoff classes, early conclusive bounded evidence, terminal-loop normalization, manifest-relative execution, malformed/unknown properties, unsupported fairness/product limits, and built-binary JSON outcomes.

Implementation candidate `d4a172cf354c23ff38c3b12197783390e43bb7b7` passed CI #615 (format, all-target build, Clippy with `-D warnings`, full tests including M73 direct/binary coverage, and every historical CLI gate) and Bounded state-property CLI #465. Closure metadata changes must pass the same exact-head gates before merge.

M73 does not claim CTL suite/expectation orchestration, CTL fairness, symbolic checking, or globally shortest CTL evidence.

## Next frontier — Milestone 74: deterministic CTL verification suites and expectations

Promote the sealed single-job CTL protocol into the existing deterministic verification-suite/regression-expectation layer without changing CTL semantics or the schema-v3 job payload.

Acceptance criteria:

- execute CTL jobs through the existing verification-suite runner rather than introducing a CTL-specific orchestrator;
- preserve each nested schema-v3 CTL job envelope byte-for-byte apart from suite framing and manifest identity;
- raw suites must retain `satisfied`, `violated`, `inconclusive`, and `error` outcomes without reinterpretation;
- expectation-aware suites must compare only the explicit expected outcome and must not discard CTL lower/upper counts, per-initial three-valued data, cutoff provenance, or evidence;
- support heterogeneous suites mixing CTL with the already sealed verification families while preserving deterministic job order and fail-closed path handling;
- add direct-single-job-vs-suite differential regressions for complete CTL, each cutoff class, early conclusive evidence, terminal self-loop/lasso evidence, and error envelopes;
- add built-binary raw-suite and expectation-suite JSON coverage;
- do not add CTL fairness, symbolic checking, or new CTL semantics in this phase;
- preserve all historical verification, structural, reduction, fairness, M69–M73 CTL, and suite gates.

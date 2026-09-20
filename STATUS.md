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

**Status: implementation candidate complete on PR #73.**

M72 exposes the sealed M71 bounded CTL semantics through the M70 declarative named-proposition frontend without introducing another semantic engine.

Implemented contract:

- typed and textual declarative bounded adapters resolve every CTL atom against existing proposition metadata before invoking `evaluate_ctl_with_limits`;
- deterministic bounded reports render `SATISFIED`, `VIOLATED`, or explicit `INCONCLUSIVE` plus exact state/transition/depth cutoff provenance;
- reports preserve discovered/checked/explored accounting, lower/upper state-set counts, per-initial `TRUE`/`FALSE`/`UNKNOWN`, memoized-subformula counts, justified finite/lasso evidence, and proven terminal-self-loop provenance;
- `fvlab ctl file <model-path> <expression> [--max-states N] [--max-transitions N] [--max-depth N]` now supports deterministic model-space limits;
- the historical no-option path remains on the sealed M70 complete-graph frontend rather than being silently rerouted through bounded semantics;
- exit codes are 0 for conclusive satisfied, 14 for conclusive violated, 3 for inconclusive, and 2 for malformed model/formula/options or unknown proposition references;
- regressions cover direct-vs-M71 equality, exact-bound completion, state/transition/depth cutoffs, retained conclusive witness, unknown-proposition fail-closed behavior, proven terminal self-loop evidence, and built-binary satisfied/violated/inconclusive/error routes.

Implementation candidate `4f20ca9e2ff26088d1b3133d98ff1a1aadfea21b` passed CI #605 (format, all-target build, Clippy with `-D warnings`, full tests including M72 built-binary coverage, and every historical CLI gate) and Bounded state-property CLI #455. Closure metadata changes must pass the same exact-head gates before merge.

M72 adds no CTL fairness, symbolic checking, verification-job/suite orchestration, or wall-clock/performance claim.

## Next frontier — Milestone 73: reproducible CTL verification jobs

Promote the sealed M70–M72 declarative CTL surfaces into the existing versioned verification-job/result architecture without duplicating parser, bounded semantics, evidence, or cutoff logic.

Acceptance criteria:

- add a CTL verification-job analysis family that references a declarative model plus textual CTL expression and optional model-space state/transition/depth limits;
- resolve manifest-relative model paths and delegate exclusively to the sealed M70 complete path when unbounded and M72/M71 bounded path when limits are configured;
- extend the machine-readable verification result envelope with stable CTL outcomes `satisfied`, `violated`, `inconclusive`, and `error`, exact cutoff provenance, and normalized finite/lasso evidence including synthetic terminal-self-loop provenance;
- preserve lower/upper state-set counts and per-initial three-valued status for bounded CTL results without reinterpreting unknown as false;
- fail closed on malformed CTL, unknown propositions, invalid limits, unsupported fields, and model-loading errors before producing a success envelope;
- add direct-vs-job differential regressions for unbounded satisfaction/violation, each cutoff class, early conclusive bounded evidence, terminal lasso/self-loop evidence, and malformed inputs;
- add a built-binary JSON job integration gate using the existing job runner conventions;
- keep CTL suite/expectation orchestration out until the single-job protocol is sealed;
- preserve all historical verification, fairness, structural, reduction, M69–M72 CTL, and suite gates.

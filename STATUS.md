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

**Status: implementation candidate complete on PR #72.**

M71 promotes complete-graph CTL to deterministic resource-bounded exploration without interpreting an incomplete prefix as a complete Kripke structure.

Implemented contract:

- M69 complete-graph evaluation is factored over a captured graph; a complete bounded capture delegates to that exact authority rather than reimplementing CTL;
- the bounded path reuses canonical bounded graph capture, cutoff accounting, known-terminal facts, and complete successor-vector provenance;
- each typed CTL subformula is evaluated as conservative lower/upper satisfaction sets, inducing retained-state truth values `True`, `False`, or `Unknown`;
- atoms and constants are exact on retained states; negation swaps/complements lower and upper sets; Boolean composition preserves lower/upper soundness;
- existential/universal predecessor rules distinguish complete successor sets from unknown outgoing behavior, and least/greatest fixpoints propagate that uncertainty through `EX`, `AX`, `EF`, `AF`, `EG`, `AG`, `E[U]`, and `A[U]`;
- only proven terminals receive synthetic self-loops; cut or unchecked states are never silently totalized;
- all-initial queries are conclusive satisfied only when every real initial is retained and definitely true, conclusive violated when a retained real initial is definitely false, and otherwise preserve the original cutoff as `INCONCLUSIVE`;
- finite/lasso evidence is retained only for conclusive top-level cases whose path/cycle uses retained model edges or proven terminal self-loops;
- zero-state budgets cannot vacuously prove an all-initial property;
- unbounded limits collapse to the exact M69 state sets, status, accounting, memoization, and supported evidence;
- focused regressions cover depth-cut terminal uncertainty, retained existential witnesses, retained universal counterexamples, retained cycles, zero-state limits, and exact unbounded collapse;
- generated soundness coverage executes **12,288** two-state graph × proposition valuation × CTL operator × cutoff combinations; every bounded true/false or conclusive whole-query result is checked against an independent full-graph oracle.

Implementation candidate `c0172d40086774c461a42adb12e1409a5c3a5bc4` passed CI #600 (format, all-target build, Clippy with `-D warnings`, full tests including the generated bounded-CTL oracle, and every historical CLI gate) and Bounded state-property CLI #450. Closure metadata changes must pass the same exact-head gates before merge.

M71 is a semantic kernel only. It does not yet expose bounded CTL through the textual/declarative CLI, verification jobs/suites, fairness assumptions, symbolic model checking, or wall-clock/resource-performance claims.

## Next frontier — Milestone 72: bounded declarative CTL file frontend

Expose M71 through the already sealed M70 parser/declarative surface without introducing another bounded semantics.

Acceptance criteria:

- add declarative typed/text adapters that resolve all named propositions before invoking `evaluate_ctl_with_limits`;
- extend deterministic CTL reporting to render conclusive `SATISFIED` / `VIOLATED` and explicit `INCONCLUSIVE` with exact state/transition/depth cutoff provenance;
- expose `fvlab ctl file <model-path> <expression> [--max-states N] [--max-transitions N] [--max-depth N]`, preserving the existing unbounded behavior when no limits are supplied;
- retain per-initial `True` / `False` / `Unknown`, lower/upper state-set accounting, justified finite/lasso evidence, and terminal-self-loop provenance in reports;
- use exit 0 for conclusive satisfied, 14 for conclusive violated, 3 for inconclusive, and 2 for malformed model/formula/options;
- add direct-vs-M71 differential tests, exact-bound/completed-prefix tests, each cutoff class, unknown-proposition fail-closed behavior, terminal/cycle evidence, and built-binary integration;
- do not add CTL verification-job/suite orchestration until the direct bounded file frontend is sealed;
- preserve all historical verification, fairness, structural, suite, reduction, M69 oracle, M70 frontend, and M71 bounded-oracle gates.

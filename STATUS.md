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

**Status: sealed in `main` at `9b0f0c3589115f9278d4ea40d9c41e7e665f9585`.**

M73 adds CTL as a first-class family in the existing heterogeneous verification-job protocol without introducing a second parser, semantic engine, or job runner. Schema-v3 result envelopes preserve complete/bounded CTL outcomes, lower/upper state data, per-initial three-valued truth, cutoff provenance, and normalized finite/lasso evidence. Exact candidate `925b3140485e80511324669d0473e04bdbfed94f` passed CI #617 and Bounded state-property CLI #467 before integration.

## Milestone 74 — generic CTL suite/expectation integration audit

**Status: architecture requirement satisfied by the existing generic verification-suite abstraction; regression evidence is carried by PR #75.**

The M74 audit found no missing production orchestrator: verification suites already execute arbitrary verification jobs through `run_verification_job_json`, preserve the complete nested `VerificationJobResultEnvelope`, aggregate only canonical job outcomes, and compare regression expectations only against explicit `VerificationJobOutcome`. Adding a CTL-specific suite engine would duplicate an already-correct abstraction.

Integration evidence added with the next production milestone proves:

- raw suites preserve schema-v3 CTL envelopes exactly against direct single-job execution;
- `satisfied`, `violated`, `inconclusive`, and `error` remain ordinary generic suite outcomes;
- expectation mode compares only explicit expected outcomes and retains CTL cutoff, lower/upper state counts, per-initial truth, and evidence;
- heterogeneous suite ordering remains deterministic when CTL and existing verification families are mixed;
- built `fvlab-suite` raw and expectation JSON embed the direct CTL job payloads without a CTL-specific path.

The exact integration evidence on candidate `40cda5996b3392de7ce597fce654c8d384a3ff4e` passed CI #625 together with all historical suite gates.

## Milestone 75 — typed modal mu-calculus kernel

**Status: integration candidate complete on PR #75.**

M75 promotes the branching-time logic layer beyond the fixed CTL operator set to a general typed modal μ-calculus authority over complete finite graphs.

Implemented contract:

- typed constants, atoms, fixpoint variables, Boolean negation/conjunction/disjunction, modal diamond/box, least fixpoint `μ`, and greatest fixpoint `ν`;
- lexical nearest-binder scoping with shadowing;
- fail-closed pre-exploration validation rejects unbound variables and occurrences that are negative relative to their nearest fixpoint binder, preserving monotonic fixpoint iteration;
- complete canonical reachable-graph capture remains the only traversal authority;
- modal predecessor semantics use the same proven-terminal synthetic self-loop policy as M69 CTL;
- least fixpoints start at the empty state set and greatest fixpoints at the full retained state set, iterating deterministically to equality;
- general Boolean negation around a fixpoint is supported when bound-variable polarity remains monotone;
- `compile_ctl_to_mu` translates the complete M69 CTL surface to μ-calculus with fresh internal variables;
- focused regressions cover validation, lexical shadowing, native μ/ν execution, terminal totalization, general negation, and nested CTL compilation;
- generated CTL→μ differential coverage performs **90,112** comparisons over all 512 directed three-state graphs: 24,576 unary-operator comparisons plus 65,536 `E[U]`/`A[U]` comparisons.

Implementation candidate `40cda5996b3392de7ce597fce654c8d384a3ff4e` passed CI #625 (format, all-target build, Clippy with `-D warnings`, full tests including the 90,112 CTL→μ differential and M74 suite-integration regressions, plus every historical CLI gate) and Bounded state-property CLI #475. Closure metadata changes must pass the same exact-head gates before merge.

M75 is a typed complete-graph semantic kernel. It does not yet expose a textual/declarative μ-calculus language, bounded-prefix μ-calculus, μ-calculus fairness, verification jobs/suites, symbolic model checking, or a performance claim from fixpoint iteration counts.

## Next frontier — Milestone 76: textual/declarative modal mu-calculus frontend

Promote the sealed typed M75 authority to an external named-proposition language without duplicating its semantics.

Acceptance criteria:

- define a deterministic textual grammar for constants, quoted proposition atoms, variables, `not`, `and`, `or`, modal diamond/box, `mu`/ `nu` binders, explicit grouping, and stable canonical rendering;
- make lexical binding and precedence unambiguous, including nested binder shadowing;
- parse first, then run the M75 binding/monotonicity validator before any model graph capture;
- bind formula atoms to existing declarative named propositions and fail closed on unknown propositions before backend exploration;
- execute exclusively through `evaluate_mu`; introduce no second fixpoint or graph engine;
- add a direct complete-graph model-file CLI/report surface with stable all-initial satisfaction/violation exits and explicit terminal policy/accounting;
- add parser round-trip/error, unbound/non-monotone, shadowing, terminal, nested alternation, direct-vs-kernel, and built-binary regressions;
- differentially verify the textual CTL-compatible subset by parsing representative μ encodings and comparing them with M75 typed execution;
- keep bounded μ-calculus, μ verification jobs/suites, fairness, and symbolic algorithms out of this phase;
- preserve all historical verification, suite, reduction, fairness, CTL, and M75 generated differential gates.

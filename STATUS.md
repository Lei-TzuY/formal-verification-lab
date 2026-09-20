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

**Status: implementation candidate complete on PR #71.**

M70 exposes the sealed M69 authority to existing declarative named propositions without introducing a second CTL semantics.

Implemented contract:

- deterministic textual grammar with quoted proposition atoms, `true`/`false`, `not`, `and`, `or`, `EX`, `AX`, `EF`, `AF`, `EG`, `AG`, `E[U]`, and `A[U]`;
- explicit Boolean precedence, bracketed until forms, UTF-8 byte-positioned parse errors, escaped proposition strings, and stable fully parenthesized canonical rendering;
- empty proposition atoms fail closed in the parser;
- all referenced atoms are resolved against existing declarative proposition metadata before the M69 backend captures/explores the model;
- `check_declarative_ctl` delegates semantic evaluation exclusively to M69 `evaluate_ctl`; all-initial satisfaction defines the direct query result;
- deterministic reporting preserves complete-graph accounting, memoized-subformula counts, per-initial satisfaction, finite/lasso evidence, real model actions, and synthetic terminal-self-loop provenance;
- direct `fvlab ctl file <model-path> <expression>` execution returns 0 for satisfied, 14 for violated, and the existing malformed-input exit 2;
- regressions cover parser precedence/round-trip/malformed forms, unknown propositions, direct-vs-M69 equality, existential/universal branching separation, terminal totalization, nested/until semantics, and built-binary satisfied/violated/error paths.

Implementation candidate `62644ff99409b3a87313c63a7f4e40fc56978024` passed CI #593 (format, all-target build, Clippy with `-D warnings`, full tests including built-binary M70 coverage, and every historical CLI gate) and Bounded state-property CLI #443. Closure metadata changes must pass the same exact-head gates before merge.

M70 remains complete-graph CTL. It makes no bounded-prefix CTL proof, CTL fairness, symbolic model checking, CTL verification-job/suite, or globally shortest-evidence claim.

## Next frontier — Milestone 71: proof-honest bounded CTL semantics

Promote the complete-graph M69/M70 authority to resource-bounded exploration without treating an incomplete graph prefix as a complete Kripke structure.

Acceptance criteria:

- reuse the canonical bounded reachable-graph capture and its completeness/known-terminal provenance; do not add a second traversal engine;
- define a sound three-valued or lower/upper satisfaction semantics for arbitrary nested CTL formulas over incomplete prefixes, so conclusive truth/falsehood is distinguished from `INCONCLUSIVE`;
- unknown outgoing behavior at cut states must not be silently totalized as a terminal self-loop;
- exact atomic valuations on retained states plus Boolean negation/composition must preserve sound lower/upper bounds;
- branching predecessor and least/greatest fixpoint operators must propagate incomplete-successor uncertainty conservatively;
- retain conclusive finite/lasso evidence only when every edge/terminal fact used by that evidence is justified by the retained graph/provenance;
- unbounded limits must collapse exactly to the sealed M69 result on state sets/status/evidence where the bounded API reports conclusive;
- add an independent generated small-graph × cutoff oracle plus focused state/transition/depth-cutoff, terminal-unknown, negation, nested-operator, and witness-provenance regressions;
- keep parser/CLI bounded exposure out until the bounded semantic kernel is sealed;
- preserve all historical verification, fairness, structural, suite, reduction, M69 oracle, and M70 frontend gates.

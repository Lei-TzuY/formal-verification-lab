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

**Status: sealed in `main` at `8842c1ebd47499f3adae8d48db1abb9cacd9f31d`.**

M75 adds a typed complete-graph modal μ-calculus authority with lexical μ/ν binding, fail-closed unbound/non-monotone-variable validation, deterministic fixpoint iteration, the shared terminal self-loop policy, and a semantics-preserving CTL→μ compiler. The generated cross-semantic gate performs 90,112 CTL→μ comparisons over all 512 directed three-state graphs. Exact candidate `40cda5996b3392de7ce597fce654c8d384a3ff4e` passed CI #625 and Bounded state-property CLI #475 before integration.

## Milestone 76 — textual/declarative modal mu-calculus frontend

**Status: sealed in `main` at `2752d891308b29859a0d6640c91e5eaf5ab9b03e`.**

M76 exposes the sealed M75 authority through deterministic textual μ-calculus syntax, declarative named-proposition binding, stable complete-graph reporting, and direct `fvlab mu file` execution. Parsing and M75 lexical/monotonicity validation precede graph capture, unknown propositions fail closed, and no second fixpoint engine is introduced. Exact closure candidate `674bf8b9a08af8ea2fc3a2a7500176d2e4514faf` passed CI #633 and Bounded state-property CLI #483 before integration.

## Milestone 77 — proof-honest bounded modal mu-calculus semantics

**Status: sealed in `main` at `a806252511da8f49ed2771e221adbfd140351792`.**

M77 adds conservative lower/upper modal μ-calculus semantics over canonical bounded graph captures, lower/upper lexical fixpoint environments, proven-terminal-only totalization, zero-budget honesty, and exact complete-capture delegation to M75. An independent bitset interpreter checks 15,360 two-state graph × valuation × native μ-formula × deterministic-limit cases. Exact closure candidate `8c1ac8fbd86a35d4a8dece4fb79124b052b3b333` passed CI #638 and Bounded state-property CLI #488 before integration.

## Milestone 78 — bounded declarative modal mu-calculus file frontend

**Status: sealed in `main` at `903d5eeb94526c4769ad50b7351ff151bfc45d5a`.**

M78 exposes the sealed M77 proof boundary through the M76 parser, proposition binder, deterministic reporting, and direct `fvlab mu file` model-space limits without duplicating μ semantics. The no-option complete path remains M76-compatible; bounded execution preserves lower/upper state sets, per-initial three-valued truth, cutoff provenance, graph accounting, and fixpoint-iteration observations. Exact closure candidate `fa2b9362246b774fbec1f1a2a130f0e3aa46e861` passed CI #645 and Bounded state-property CLI #495 before integration.

## Milestone 79 — reproducible modal mu-calculus verification jobs

**Status: sealed in `main` at `79ea6a9abf32b2d9d783276f3a055686de461454`.**

M79 promotes modal μ-calculus into the existing heterogeneous verification-job protocol without adding a second parser, evaluator, or runner. `analysis "mu-calculus"` preserves manifest-relative model/property loading, rejects fairness and product-only budgets before referenced-file I/O, delegates complete jobs to M76 and model-limited jobs to M77, and normalizes results into schema-v4 envelopes. Exact closure candidate `461461cd6ac838431b0b7342646cd5df2614c379` passed CI #653 and Bounded state-property CLI #503 before squash integration.

## Milestone 80 — generic modal mu-calculus suite integration audit

**Status: architecture requirement satisfied by the existing generic verification-suite abstraction; executable regression evidence is carried by PR #80.**

The M80 audit found no missing production orchestrator. Verification suites already execute arbitrary jobs through `run_verification_job_json`, retain the complete nested `VerificationJobResultEnvelope`, aggregate only canonical four-state outcomes, and compare expectations only against explicit `VerificationJobOutcome`. A μ-specific suite engine would duplicate an already-correct abstraction.

Integration evidence carried with M81 proves:

- raw suites preserve schema-v4 μ envelopes exactly against direct single-job execution;
- `satisfied`, `violated`, `inconclusive`, and `error` remain ordinary generic suite outcomes;
- expectation mode compares only explicit expected outcomes while retaining μ cutoff, lower/upper state counts, per-initial truth, and fixpoint details;
- heterogeneous suite ordering remains deterministic when μ jobs are mixed with existing verification families;
- built `fvlab-suite` raw and expectation JSON embed direct μ job payloads without a μ-specific path.

The evidence is part of the M81 exact candidate and therefore passes or fails with the substantive production milestone rather than existing as a test-only micro-PR.

## Milestone 81 — modal mu-calculus parity-game backend

**Status: sealed in `main` at `28690d7dd44e8ce71af0dcc3210c705ab08e7ae4`.**

M81 adds an independent complete-graph μ-calculus evaluation-game backend alongside the sealed M75 direct state-set fixpoint evaluator. A validated finite parity-game kernel computes deterministic Even/Odd winning regions with Zielonka recursion; `evaluate_mu_via_parity` validates formulas before graph capture, normalizes general negation by dualizing Boolean/modal/fixpoint structure, reuses the canonical terminal-self-loop policy, and derives priorities from lexical μ/ν alternation. The generated differential covers 3,584 two-state graph × valuation × native μ-formula cases. Exact closure candidate `6e9a9f41744320ed43bc4aabd5fc1c9009976134` passed CI #665 and Bounded state-property CLI #515 before squash integration.

## Milestone 82 — opt-in declarative modal mu-calculus parity backend

**Status: sealed in `main` at `12a570cad782e76218e5c5f36213fdac7f09d32d`.**

M82 exposes the sealed M81 parity authority through the existing declarative named-proposition frontend and direct `fvlab mu file` command without replacing the default M75/M76 fixpoint path. `mu file ... --backend parity` is explicit opt-in; no-option execution remains the historical fixpoint route, and parity combined with model-space limits fails closed rather than claiming bounded parity semantics. Exact closure candidate `68b76c6b7c24d3063eee7980d885388185618f88` passed CI #671 and Bounded state-property CLI #521 before squash integration.

## Milestone 83 — reproducible parity-backed modal mu-calculus verification jobs

**Status: sealed in `main` at `47005a56da70e4edef4d09591a566a8309528b7e`.**

M83 carries M82's explicit complete parity backend into the existing M79 heterogeneous verification-job protocol while preserving fixpoint as the default and bounded authority. The manifest adds `backend "fixpoint"|"parity"` only for `analysis "mu-calculus"`; absent metadata preserves historical fixpoint documents, parity plus model-space limits fails closed before referenced-file I/O, schema-v4 retains backend-specific observations without fabricating fixpoint iterations, and generic suites preserve mixed backend envelopes. Exact closure candidate `81f2c7619792032e2e6193bc7bdd0e9b054b6d92` passed CI #679 and Bounded state-property CLI #529 before squash integration.

## Milestone 84 — certified positional parity strategies

**Status: sealed in `main` at `e8c6571d805020194095c8bcf69e84427316ea19`.**

M84 promotes the sealed M81 parity kernel from winning-region classification to deterministic positional strategy evidence with an independent fail-closed verifier while preserving the historical region API. Solver strategies are composed through Zielonka recursion, and verification independently checks ownership, real-edge selection, region closure, totality, and recurrent parity. The generated certification gate covers all 324 two-vertex total games across owner assignments, priorities 0..2, and non-empty successor subsets. Exact closure candidate `af818c5064c3e3c6c177e37498f6eafb5b92d245` passed CI #686 and Bounded state-property CLI #536 before squash integration.

## Milestone 85 — typed modal mu-calculus parity strategy evidence

**Status: implementation candidate complete on PR #84.**

M85 bridges the sealed M84 generic strategy certificates back into the M81 μ-calculus evaluation game without changing schema-v4, verification jobs, suites, or CLI behavior.

Implemented contract:

- `MuParityEvaluation<S>` keeps its existing generic shape and adds typed normalized evaluation-game positions rather than introducing atom/variable generic parameters into downstream declarative/job APIs;
- each position records reachable model-state index, normalized formula-node index/kind, owner, and priority;
- certified Even/Odd strategies are mapped to semantic moves: Boolean left/right branch, model successor, terminal modal self-loop, fixpoint body, bound-variable return, or outcome self-loop;
- every initial root position records its truth and certified winning player;
- `evaluate_mu_via_parity` now verifies the M84 Even and Odd strategies before exposing typed evidence;
- `verify_mu_parity_evidence` validates formula binding/monotonicity first, recaptures the canonical complete graph, rebuilds the normalized evaluation game, checks canonical position metadata and semantic move labels, reconstructs generic `ParityStrategy` values, delegates parity correctness to the sealed M84 verifier, requires the two winning regions to partition the game, and revalidates satisfying/initial state results and model accounting;
- focused regressions cover outcome, Boolean, modal successor, terminal modal self-loop, fixpoint-body and variable-return moves, plus nested alternation and lexical shadowing;
- tamper regressions reject altered position kinds, semantic move labels, missing winner moves, incomplete winning partitions, and altered initial winners;
- a generated 3,584-case two-state graph × P/Q valuation × native μ-formula gate requires the typed evidence verifier to accept every produced certificate and requires truth/initial results to match M75/M81.

Implementation candidate `4d4a256d2af1b757a61505a1c5ea3dd250836fd0` passed CI #690 (format, all-target build, Clippy with `-D warnings`, full tests including the generated M85 evidence gate and all historical CTL/μ/parity/job/suite tests, plus every historical CLI gate) and Bounded state-property CLI #540. Closure metadata changes must pass the same exact-head gates before merge.

M85 remains typed/in-memory and makes no portable proof-certificate, bounded-parity, symbolic, fairness, or performance claim.

## Next frontier — Milestone 86: versioned declarative modal mu-calculus parity certificate

Promote the sealed M85 typed evidence into a deterministic, versioned artifact only for the declarative complete-parity surface. Do not expose raw Rust debug output or silently reinterpret schema-v4 verification-job results as proof certificates.

Acceptance criteria:

- define an explicit certificate schema/version whose fields are derived from M85 typed position/strategy/initial evidence and are sufficient to reconstruct that evidence without relying on Rust type/debug formatting;
- bind the artifact to the canonical declarative model identity/state ordering and canonical rendered μ formula; reject certificate reuse against a different model/formula;
- deterministic render + parse must round-trip exactly and preserve strategy player, winning positions, semantic choices, priorities, and initial-root winners;
- certificate verification must parse/validate the model and μ formula through the sealed M76/M82 frontend, rebuild the canonical complete evaluation game, reconstruct M85 typed evidence, and invoke `verify_mu_parity_evidence`; it must not call the parity solver to decide whether the submitted certificate is correct;
- malformed, truncated, duplicate, out-of-range, wrong-model, wrong-formula, and semantically tampered certificates must fail closed with stable errors;
- add an explicit direct CLI verification path for certificate files only after the library parser/verifier is sealed; keep ordinary `mu file` output and M83 verification-job schema unchanged in this milestone;
- add deterministic parser/render roundtrip, independent verification, tamper, manifest-relative/built-binary, and representative alternation/shadowing regressions;
- preserve M75's 90,112-case CTL→μ differential, M77's 15,360-case bounded oracle, M81/M85's parity/evidence generated gates, M84's strategy certification gate, and every historical job/suite/CLI gate;
- make no cryptographic authenticity, bounded-parity, symbolic, fairness, or performance claim.


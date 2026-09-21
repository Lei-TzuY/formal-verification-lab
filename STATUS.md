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

**Status: sealed in `main` at `1a26321cb56deaaef6b13ad3232fa4c986ead75b`.**

M85 maps the sealed M84 generic positional strategies back into typed modal μ-calculus evaluation-game positions and semantic moves, associates initial roots with certified winners, and adds `verify_mu_parity_evidence`, which reconstructs the canonical evaluation game and delegates strategy correctness to M84 without trusting rendered evidence. Its 3,584-case generated evidence gate agrees with M75/M81 truth results. Exact closure candidate `100112bee60cad58a7eae4f13ce9ad0c5ef0119b` passed CI #692 and Bounded state-property CLI #542 before squash integration.

## Milestone 86 — versioned declarative modal mu-calculus parity certificate

**Status: sealed in `main` at `7176714269c62c7d50795075578a7cf747dbe317`.**

M86 promotes M85's typed in-memory evidence into an explicit deterministic schema-v1 artifact for the declarative complete-parity surface while keeping ordinary `mu file`, M83 verification-job schema-v4, bounded μ semantics, and the default fixpoint path unchanged. Certificates bind canonical validated model declarations plus canonical rendered μ formula, retain typed positions/strategies/initial winners, round-trip deterministically, and are independently verified by rebuilding M85 evidence without invoking the parity solver. Direct `fvlab mu certificate create` / `verify` commands expose the sealed surface. Exact closure candidate `575d267d3a03a953ba389beb0a53b31a93a592ef` passed CI #706 and Bounded state-property CLI #556 before squash integration.

M86 makes no cryptographic authenticity, signature, trust-chain, bounded-parity, symbolic, fairness, or performance claim.

## Milestone 87 — reproducible modal mu-calculus certificate verification jobs

**Status: sealed in `main` at `e057d31bd60b6ddd0def54f98933f7178a07767c`.**

M87 promotes M86 certificate verification into a dedicated manifest-relative machine-readable job surface with explicit `verified / rejected / error` semantics, deterministic JSON, 0/16/2 exit codes, relative model/property/certificate paths, direct `fvlab mu certificate job` execution, and strict separation between setup `error` and loaded-artifact `rejected`. The runner verifies only the submitted M86 artifact and never creates or re-solves a replacement certificate. Exact closure candidate `1996fc438881e58663a05334e963dc3cff8a5fa5` passed CI #714 and Bounded state-property CLI #564 before squash integration.

M87 makes no certificate-authenticity, signature, trust-chain, bounded-parity, symbolic, fairness, or performance claim.

## Milestone 88 — outcome-neutral multi-family job orchestration

**Status: sealed in `main` at `2cea4671cba87132bc6a130d29d946022393fa49`.**

M88 adds one additive family-tagged orchestration layer above the sealed verification, structural and certificate-verification job protocols. It preserves every native nested envelope, keeps regression expectations family-aware, and separates orchestration `complete / attention / error` from domain truth/structure/artifact outcomes. Verification `violated` and structural `cycle_found` remain non-error domain results; certificate `rejected` yields operational attention. Exact closure candidate `af642c99726097f80a3e704e864882b6d9235ef9` passed CI #726 and Bounded state-property CLI #576 before squash integration.

## Milestone 89 — host-independent virtual workspace execution

**Status: sealed in `main` at `0fbd6bb7330394a97e555abf1ba1f98156373bf0`.**

M89 promotes M88 from direct host-filesystem coupling to one shared logical UTF-8 text-source substrate. Verification, structural, certificate-verification and multi-family orchestration all execute through the same `TextSourceProvider` core; filesystem APIs remain compatibility wrappers, rooted and map providers enforce canonical logical ids, and provider-mode diagnostics avoid host-root leakage. Exact closure candidate `01e59a3378cbeb75dd0227e3a7db817a1e50a7eb` passed CI #736 and Bounded state-property CLI #586 before squash integration.

M89 intentionally makes no archive/bundle, digest/hash, signature, cryptographic-authenticity, filesystem-sandbox, bounded-parity, symbolic, fairness or performance claim.

## Milestone 90 — versioned portable workspace snapshot and replay

**Status: sealed in `main` at `b4828f340c0777d47c65564cf8325116245b72f1`.**

M90 freezes one explicit M88 orchestration root plus its exact transitive M89 logical text-source closure into a deterministic bounded schema-v1 UTF-8 artifact. Closure discovery follows only explicit family-tagged manifests and declared model/property/certificate dependencies; parsing reverse-validates that exact closure, rejects unrelated or missing sources, and replays raw or expectation orchestration entirely through the sealed map-backed provider after host files disappear. Cross-root and map-provider creation is byte-identical. Exact closure candidate `07f196f95e48e0aa7654bac3b3069f80c9f17976` passed CI #743 and Bounded state-property CLI #593 before squash integration.

M90 makes no digest/hash, signature, tamper-authentication, cryptographic-authenticity, compression, performance or filesystem-sandbox claim.

## Milestone 91 — versioned exact replay result lock

**Status: sealed in `main` at `c70ef3584f1975efc76084e29db0f0540a9bb1d3`.**

M91 adds a portable exact-result regression artifact above the sealed M90 input snapshot. Schema-v1 replay locks embed the exact M90 snapshot, replay mode, expected process exit code and exact deterministic replay JSON. Verification replays only the embedded snapshot and compares exit code plus JSON bytes without normalizing family outcomes. Raw and expectation modes, offline replay, deterministic framing/resource limits, outcome-preserving accounting drift and direct-library/built-CLI parity are covered. Exact closure candidate `c461c9de80324ef3e11fbb99c9276d321baa5191` passed CI #751 and Bounded state-property CLI #601 before squash integration.

M91 makes no hash, signature, cryptographic-integrity/authenticity, compression, performance or sandbox claim.

## Milestone 92 — structured exact-replay drift diagnostics

**Status: implementation candidate complete on PR #91.**

M92 makes M91 byte-level replay mismatches actionable without weakening exact comparison authority.

Implemented contract:

- M91 exit-code equality plus byte-for-byte replay JSON equality remain the sole match/mismatch authority;
- `json_difference` is diagnostic-only and is emitted only when `json_matches=false`; a diagnostic parser failure never converts a mismatch into a match or verification error;
- a bounded dependency-free JSON parser enforces maximum input bytes, recursion depth, node count and preview size;
- deterministic structural comparison reports the first stable JSON-Pointer-style path using ordered object/key traversal;
- typed drift kinds cover scalar value changes, type changes, missing/unexpected object members, array length changes, structurally-equal-but-byte-different JSON and invalid/resource-limited byte fallback;
- JSON Pointer keys escape `~` and `/` as `~0` / `~1`;
- malformed or diagnostic-resource-limited JSON retains M91 mismatch semantics and falls back to the first byte offset;
- exit-code drift remains independently visible and does not invent a JSON difference when JSON bytes still match;
- the M91 accounting mutation is diagnosed at `/jobs/0/result/accounting/model_states` while the nested verification outcome remains `satisfied`;
- regressions cover nested-array value drift, missing/unexpected members, type changes, array length, pointer escaping, structurally-equal byte drift, invalid JSON fallback, matched runs, exit-only drift and direct-library/built-`fvlab-workspace lock-verify --format json` equality;
- M91 lock schema-v1 bytes, M90 snapshot schema/replay, M89 providers, M88 orchestration and all family result schemas remain unchanged.

The adopted pre-existing M92 branch initially failed CI #753/#755/#756 only on rustfmt while Bounded CLI builds remained healthy; formatter output was applied without changing semantics. Exact implementation candidate `eb7b629b001761b29d058a007cf18aed7808c5d9` passed CI #757 (format, all-target build, Clippy with `-D warnings`, full M92 diagnostics coverage plus every historical CTL/μ/parity/certificate/job/suite/orchestration/workspace/replay-lock gate and historical CLI smoke test) and Bounded state-property CLI #607. Closure metadata changes must pass the same exact-head gates before merge.

M92 makes no digest/hash, signature, cryptographic-authenticity, performance, compression or sandbox claim.

## Next frontier — Milestone 93: bounded deterministic replay audit

M91/M92 assume the frozen M90 workspace replay itself is deterministic when interpreting exact-result drift, but replay-lock verification currently performs only one fresh replay. Lower-level engines have many determinism regressions; the portable workspace/replay boundary does not yet expose an executable repeatability audit.

Acceptance criteria:

- add a bounded repeatability API over one parsed M90 `WorkspaceSnapshot` and explicit replay mode (`raw` or `expectations`);
- execute one canonical baseline replay plus a caller-specified bounded number of additional attempts; reject zero/overflow/excessive attempt counts before execution;
- compare every attempt against the first using the same exit-code + exact JSON-byte authority as M91, with no normalization of family-native outcomes;
- report `deterministic / nondeterministic / error` independently from M91 lock match/mismatch semantics;
- on the first nondeterministic attempt, preserve baseline/current exit codes and byte-equality flags and attach the sealed M92 structured JSON diagnostic when JSON differs;
- stop at the first observed mismatch rather than farming later repetitions, and report the 1-based attempt index deterministically;
- support both raw and expectation replay modes entirely from the sealed snapshot/map provider, with no host filesystem dependency after parsing;
- add explicit resource bounds on repeat count; do not use wall-clock timing, CI duration or repeated execution as a performance benchmark;
- add direct library and `fvlab-workspace` CLI JSON surfaces with byte-identical output/exit behavior;
- regressions must cover stable raw/expectation snapshots, an injected deterministic test provider/hook that produces a known second-attempt JSON or exit drift, M92 diagnostic reuse, invalid attempt counts, and built-binary equality;
- preserve M92 diagnostics, M91 lock schema-v1/verification, M90 snapshot schema/replay, M89 providers, M88 orchestration and all family result schemas;
- make no performance, probabilistic-guarantee, digest/hash, signature, cryptographic-authenticity or sandbox claim.


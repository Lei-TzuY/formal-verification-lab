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

**Status: implementation candidate complete on PR #89.**

M90 packages one sealed M88 orchestration root plus its exact transitive M89 logical text-source closure into a deterministic standalone schema-v1 artifact and replays it without the original host workspace.

Implemented contract:

- snapshot creation accepts any M89 `TextSourceProvider`, normalizes one orchestration root id, parses the explicit M88 family tags, parses each referenced family manifest, and follows only declared model/property/certificate dependencies;
- no directory walking, wildcard capture, or family inference is performed;
- sources are stored in deterministic `BTreeMap` logical-id order and rendered with explicit byte-length framing for root ids, source ids and exact UTF-8 source text;
- embedded newlines, quotes, backslashes and Unicode are preserved exactly without serde/base64/dependency churn;
- schema-v1 parsing checks header/version, frame boundaries, UTF-8 boundary validity, required newline terminators, declared entry/source-byte totals, canonical logical ids, duplicates, required root/dependency closure and unrelated extra sources;
- build/parser resource limits bound source count, aggregate embedded source bytes and source-id length before unbounded internal allocation;
- parsed artifacts are reverse-validated against the embedded orchestration/family manifests so the source set must equal the explicit dependency closure;
- replay reconstructs an M89 `MapTextSourceProvider` and runs raw or expectation M88 orchestration exclusively from embedded texts;
- snapshot creation from two different rooted physical workspaces and from an in-memory map containing the same logical workspace produces byte-identical artifacts;
- removing the original host workspace after snapshot creation does not change raw or expectation replay results;
- regressions cover satisfied, violated, recurrence-cycle and verified results, loaded-certificate rejection, setup error, exact render/parse roundtrip, noncanonical ids, root escape, duplicate/missing/unrelated sources, truncation, trailing payload, resource caps and UTF-8 frame splits;
- one built surface, `fvlab-workspace`, creates snapshots and replays raw or expectation orchestration; built CLI stdout/exit is differential-checked against direct library replay.

Initial CI exposed only rustfmt differences plus one Rust type-inference ambiguity in the dependency-parser error prefix. The latter was fixed by making the existing `String` error type explicit; no closure, resource-limit, replay or test semantics changed.

Exact implementation candidate `ec92e910c54481859b9135842eecd1616ac77935` passed CI #741 (format, all-target build, Clippy with `-D warnings`, full tests including M90 snapshot/replay/built-CLI regressions plus every historical CTL/μ/parity/certificate/job/suite/orchestration/virtual-workspace gate and historical CLI smoke tests) and Bounded state-property CLI #591. Closure metadata changes must pass the same exact-head gates before merge.

M90 makes no digest/hash, signature, tamper-authentication, cryptographic-authenticity, compression, performance or filesystem-sandbox claim.

## Next frontier — Milestone 91: versioned exact replay result lock

M90 freezes all execution inputs, but M88 expectation mode intentionally compares only each family-native outcome. A replay can therefore preserve `satisfied`/`violated`/`cycle_found`/`verified` while deterministic evidence, accounting, backend metadata or nested result-schema details drift. M91 should add a portable exact-result regression artifact above M90 without pretending that byte equality is cryptographic authenticity.

Acceptance criteria:

- define a versioned standalone replay-lock artifact containing one exact M90 snapshot, an explicit execution mode (`raw` or `expectations`), the expected process exit code, and the exact deterministic JSON produced by that snapshot in that mode;
- create a lock only by successfully parsing/validating the M90 snapshot and executing it through the sealed M90 replay API; do not synthesize or normalize the expected JSON independently;
- use explicit length framing and bounded parser limits for embedded snapshot/result payloads; preserve their bytes exactly and reject truncation, trailing payload, unsupported versions, invalid mode, count/length overflow and malformed embedded snapshots fail-closed;
- lock verification must replay the embedded snapshot through the current engine and compare both exit code and JSON byte-for-byte, without parsing/re-serializing the expected JSON;
- expose explicit `matched / mismatched / error` verification semantics; a valid lock whose current replay differs is a mismatch, while malformed lock/snapshot material is an error;
- mismatch evidence must identify whether exit code, JSON, or both differ without claiming why the engine changed;
- creating the same lock from byte-identical M90 snapshots produced under different host roots/providers must yield byte-identical lock artifacts;
- lock verification must remain fully offline after the original workspace is removed;
- regressions must prove that outcome-preserving mutations to expected full JSON (for example accounting/evidence fields) are detected even when the family-native outcome string is unchanged;
- support both raw and expectation M90 replay modes and preserve the complete nested result JSON rather than inventing another domain-outcome abstraction;
- add direct library APIs first, then extend the existing `fvlab-workspace` binary with lock create/verify commands and direct-library vs built-binary differentials;
- preserve M90 snapshot schema/replay behavior, M89 provider APIs, M88 orchestration schemas/exits and all family result schemas;
- make no hash, signature, cryptographic-integrity/authenticity, compression, performance or sandbox claim.


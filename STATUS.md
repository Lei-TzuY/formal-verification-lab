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

**Status: integration candidate complete on PR #70.**

M69 adds a parser-free typed CTL semantic authority over one complete canonical reachable graph.

Implemented contract:

- typed Boolean formulas plus `EX`, `AX`, `EF`, `AF`, `EG`, `AG`, `E[U]`, and `A[U]`;
- explicit terminal policy: every reachable terminal is totalized with one synthetic self-loop for CTL path semantics while the original model graph remains unchanged;
- deterministic least/greatest fixpoint evaluation over one captured `ReachableGraph`;
- structural subformula memoization so repeated nested formulas reuse their computed state sets instead of re-running model traversal;
- deterministic finite evidence for existential `EX`/`EF`/`E[U]` satisfaction and universal `AX`/`AG`/`A[U]` failure where the kernel can justify it directly;
- deterministic lasso evidence for `EG` satisfaction and `AF`/`A[U]` nontermination counterevidence;
- evidence explicitly distinguishes real model actions from synthetic terminal self-loops;
- focused terminal, branching, nested-formula, memoization, duality, finite-evidence, and lasso regressions;
- generated independent path-oracle coverage across all 512 directed three-state graphs: 4,096 unary proposition valuations checked across six unary CTL operators, plus 32,768 binary proposition valuations checked across both until operators.

Implementation candidate `78912262f6a37733ce3e3f03b90030f86e91344f` passed CI #587 (format, all-target build, Clippy with `-D warnings`, full tests including generated CTL oracles, and every historical CLI gate) and Bounded state-property CLI #437. Closure metadata changes must pass the same exact-head gates before merge.

M69 is a typed semantic kernel, not yet a textual/declarative CTL frontend. It makes no arbitrary CTL parser, bounded-prefix CTL proof, symbolic model checking, or shortest-witness claim.

## Next frontier — Milestone 70: declarative CTL parser and file frontend

Promote the sealed typed M69 authority to an external named-proposition surface without introducing a second CTL semantics.

Acceptance criteria:

- deterministic textual grammar for Boolean composition plus `EX`, `AX`, `EF`, `AF`, `EG`, `AG`, `E[U]`, and `A[U]`, with explicit precedence/grouping and canonical rendering;
- bind CTL atoms to existing declarative named state propositions and fail closed on unknown proposition references before backend execution;
- execute through M69 `evaluate_ctl` only, preserving its terminal self-loop policy, state-set semantics, memoization, and evidence;
- add a declarative model-file CLI with stable report/exit behavior for all-initial satisfaction versus violation and malformed input;
- preserve deterministic evidence rendering, including synthetic terminal-loop provenance, without claiming shortest traces;
- parser round-trip, malformed grammar, unknown proposition, terminal/cycle, nested operator, direct-vs-frontend, and built-binary integration regressions;
- keep fairness external and unsupported for CTL in this phase; add no CTL job/suite protocol until the direct file frontend is sealed;
- preserve all historical safety, temporal, fairness, structural, suite, reduction, and M69 oracle gates.

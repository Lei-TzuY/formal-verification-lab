# Formal Verification Lab

A serious educational laboratory for building formal-methods machinery from first principles in Rust.

The project is an explicit-state verification lab, not a wrapper around an existing model checker and not production-grade verification software. The emphasis is executable semantics, deterministic witnesses, independent graph oracles, honest resource bounds, and explicit trust boundaries.

## Current capability

The repository now has one coherent explicit-state stack from finite transition-system construction through safety, reachability, recurrence, liveness, response obligations, generalized Büchi acceptance, textual/declarative frontends, deterministic model/product resource budgets, deep-graph SCC traversal, and opt-in exact-action weak fairness.

Historical no-fairness behavior remains the default. Weak fairness is enabled only when explicitly supplied, and it constrains infinite executions only. Product and staged cutoffs remain proof-honest: missing prefix edges are never treated as proof that an action is disabled, and unresolved enablement provenance produces `INCONCLUSIVE` rather than a false proof or counterexample.

## Implemented milestones

| Milestone | Capability |
| --- | --- |
| M1 | Deterministic explicit-state safety checking with shortest BFS counterexamples. |
| M2 | State/transition/depth exploration limits with honest `INCONCLUSIVE`. |
| M3 | Typed transition-system construction and executable Peterson mutual-exclusion models. |
| M4 | Exploration diagnostics plus an independent 512 directed three-state graph oracle. |
| M5 | Experimental sleep-set reduction guarded by differential exhaustive auditing. |
| M6 | Existential reachability with deterministic shortest witnesses. |
| M7 | Deadlock/legitimate-terminal analysis with shortest witnesses. |
| M8 | Reachable SCC analysis and deterministic recurrent-cycle witnesses. |
| M9 | Universal eventuality over maximal executions with finite or lasso counterexamples. |
| M10 | Single action-response obligations. |
| M11 | Multi-class response obligations with per-clause pending semantics. |
| M12 | Generic deterministic finite-monitor products. |
| M13 | Deterministic generalized Büchi-style acceptance with explicit finite-run policy. |
| M14 | Shared deterministic action-product construction. |
| M15 | Response engines migrated onto the shared product substrate. |
| M16 | Neutral captured reachable-graph ownership shared by graph/temporal analyses. |
| M17 | Typed exact-action temporal frontend. |
| M18 | Textual parser for `response(...)` and `infinitely-often(...)`. |
| M19 | Declarative finite labeled-graph model files. |
| M20 | Exact-state reachability/eventuality frontend. |
| M21 | Declarative named state propositions. |
| M22 | Boolean proposition expressions with `not`, `and`, and `or`. |
| M23 | Declarative Boolean safety assertions. |
| M24 | Bounded state-property verification with honest incompleteness. |
| M25 | Product-bounded single/multi-response verification. |
| M26 | Product-bounded finite-monitor and generalized Büchi verification. |
| M27 | Product-bounded typed/textual/declarative temporal frontend. |
| M28 | Independent model-space and response-product budgets with stage-qualified outcomes. |
| M29 | Staged model/product budgets across monitor, Büchi, and temporal frontends. |
| M30 | Iterative Tarjan SCC traversal validated against the previous recursive semantics and 50,000-node graphs. |
| M31 | Explicit exact-action weak-fairness liveness core. |
| M32 | External `--weak-fair-action` assumption surface for temporal verification. |
| M33 | Product-bounded and staged weak fairness with per-state action-enablement provenance. |
| M34 | Weak-fair single-response obligations across backend, typed/textual/declarative frontend, and bounded/staged CLI paths. |

### M31 — explicit weak-fairness liveness core

`WeakFairness` stores an ordered, validated set of exact action labels. For every configured action `a`, an admitted infinite execution may not postpone `a` forever while `a` remains continuously enabled.

`check_buchi_with_weak_fairness` reuses the generalized Büchi and recurrent-graph machinery. Fairness is checked against **full-product action enablement**, not only the property residual: an edge that leaves an acceptance-avoiding residual still proves that its action is enabled. A recurrent SCC can satisfy a weak-fair obligation when some recurrent state disables the action or an internal recurrent edge actually takes it. Returned fair counterexamples are deterministic closed walks that contain the required disabled-state or taken-edge evidence.

Finite terminal policy is unchanged because weak fairness constrains only infinite executions. The empty fairness set is an exact compatibility path. M31 adds no strong fairness, implicit scheduler fairness, arbitrary temporal-logic fairness, or performance claim.

### M32 — external weak-fairness assumption surface

Repeated `--weak-fair-action <ACTION>` declarations expose exact-action weak fairness through fixed teaching models, textual `temporal check`, and declarative `temporal file` routes. Declaration order is preserved and duplicate/malformed assumptions fail closed.

Fairness assumptions are rendered explicitly and separately from the property report. No-option invocations preserve historical no-fairness behavior. M32 established the external assumption surface; later milestones compose that surface with bounded/staged analysis and response obligations.

### M33 — bounded/staged weak fairness with enablement provenance

M33 composes weak fairness with product-only and staged model/product limits without inferring disabled actions from missing prefix edges. Bounded graph/product construction carries per-state exact-action enablement provenance so fair recurrent analysis can distinguish:

- a real taken fair-action edge;
- a state whose complete outgoing relation proves the fair action disabled; and
- a cutoff that leaves enablement unknown.

A retained recurrent witness remains conclusive only when every required fairness obligation is justified by real taken-edge or proven-disabled evidence. If a model/product cutoff leaves required enablement unknown, the result is `INCONCLUSIVE` with the exact stage and state/transition/depth reason. Generous limits preserve the unbounded M31 semantics.

The fixed, textual, and declarative recurring-action temporal paths all share this behavior. M33 adds no strong fairness, wall-clock timeout, total-memory bound, or performance claim.

### M34 — weak-fair single-response obligations

M34 extends the same fairness semantics to the canonical single-response property instead of creating a second response-specific fairness traversal.

A response obligation is compiled to a deterministic pending-bit generalized Büchi automaton:

- initial control state: `pending = false`;
- response action: clear `pending`;
- otherwise trigger action: set `pending`;
- acceptance set: `!pending`;
- finite policy: `RequireAcceptingTerminal`.

This preserves the response contract: a finite maximal execution that terminates while a request is pending is still a real violation, because weak fairness does not rewrite finite executions. For infinite executions, a continuously enabled fair response can exclude a lasso that postpones that response forever. Fairness on an action that is actually taken by a violating cycle does not erase the genuine response counterexample.

The implementation reuses the M31/M33 weak-fair Büchi engines for unbounded, product-only, and staged analysis. `check_response_with_weak_fairness`, `check_response_with_weak_fairness_and_product_limits`, and `check_response_with_weak_fairness_and_limits` preserve the established response result/accounting surfaces. An empty fairness set delegates exactly to the historical response paths.

The typed `ActionTemporalSpec::response`, textual `response("trigger","response")`, fixed request/grant teaching model, and declarative file routes all normalize back to `TemporalBackend::Response`; the Büchi compilation remains an internal semantic implementation detail. Product/model cutoffs preserve M33 enablement provenance and exit 3 when unresolved; real response violations retain exit 10 through the temporal frontend.

Executable regressions cover matching versus taken/unrelated fairness, finite pending terminals, exact empty-fairness compatibility, product cutoff honesty, staged model cutoff provenance, retained fair cycles, fixed/textual/declarative CLI routing, and deterministic fair response satisfaction.

## Architecture

```text
src/model.rs                  transition-system abstraction and validation
src/builder.rs                typed construction layer
src/checker.rs                canonical deterministic BFS substrate and bounds
src/bounded.rs                bounded + stage-qualified whole-analysis outcomes
src/bounded_report.rs         stable bounded-cutoff reason formatting
src/declarative.rs            external graph parser, canonical materialization,
                              proposition metadata ownership
src/graph.rs                  neutral captured labeled graphs, bounded/unbounded capture,
                              enablement provenance, accounting, shortest paths
src/product.rs                bounded/unbounded + staged captured-model-to-product BFS
src/property.rs               existential reachability + deadlock policies
src/recurrence.rs             iterative Tarjan SCCs, cyclic classification, cycle witnesses
src/fairness.rs               exact-action weak fairness + fair recurrent witnesses
src/bounded_fairness.rs       product-bounded/staged weak-fair Büchi composition
src/fairness_report.rs        explicit weak-fair temporal assumption reporting
src/eventuality.rs            universal eventuality over target-cut residuals
src/multi_response.rs         product-bounded + staged multi-clause response semantics
src/response.rs               no-fair + weak-fair single-response adapters
src/monitor.rs                unbounded, product-bounded + staged finite-monitor semantics
src/buchi.rs                  unbounded, product-bounded + staged Büchi semantics
src/temporal.rs               typed response/recurring routing, including weak fairness
src/temporal_parse.rs         textual parser for the typed temporal subset
src/temporal_report.rs        normalized unbounded/product-bounded/staged reporting
src/exact_state.rs            exact-state frontend + backend routing
src/proposition.rs            named-proposition frontend + backend routing
src/proposition_expr.rs       Boolean proposition AST/parser + backend routing
src/safety.rs                 query-time Boolean safety assertion frontend
src/*_report.rs               deterministic analysis-specific reporting
src/*_examples.rs             executable teaching models
src/main.rs                   CLI/file/exit-status integration; no model traversal logic
tests/                        semantic, oracle, graph/product, frontend and CLI tests
```

The original transition relation remains owned by `TransitionSystem` plus canonical exploration. Structural and temporal analyses reuse neutral captured finite labeled graphs and one shared deterministic action-product substrate rather than invoking separate traversal engines. Weak fairness is opt-in and composes beside the recurrent graph machinery; it does not alter default exploration semantics.

## Executable examples

```bash
cargo run -- run counter
cargo run -- run mutex-bug
cargo run -- run peterson
cargo run -- reduce commuting-counters
cargo run -- reach counter-three
cargo run -- deadlock counter-terminal-forbidden
cargo run -- scc traffic-light
cargo run -- eventually counter-three

cargo run -- respond request-grant
cargo run -- respond request-grant --max-product-depth 1
cargo run -- respond request-grant --max-model-depth 1 --max-product-depth 1
cargo run -- respond dual-grant --max-product-states 4

cargo run -- monitor session-ok
cargo run -- monitor session-stuck --max-model-transitions 3 --max-product-transitions 4
cargo run -- buchi pulses
cargo run -- buchi pulses-unfair --max-model-transitions 2

cargo run -- temporal request-grant
cargo run -- temporal request-grant-unfair
cargo run -- temporal request-grant-unfair --weak-fair-action grant
cargo run -- temporal request-grant-unfair --weak-fair-action grant --max-product-transitions 2
cargo run -- temporal request-grant-unfair --weak-fair-action grant --max-model-transitions 2
cargo run -- temporal pulses-unfair --weak-fair-action pulse-b
cargo run -- temporal check request-grant-unfair 'response("request","grant")' --weak-fair-action grant
cargo run -- temporal check pulses-unfair 'infinitely-often("pulse-a","pulse-b")' --weak-fair-action pulse-b
cargo run -- temporal file path/to/model.fvl 'response("request","grant")' --weak-fair-action grant

cargo run -- state file path/to/model.fvl 'reachable("done")' --max-depth 2
cargo run -- proposition file path/to/model.fvl reachable critical --max-states 20
cargo run -- proposition expr path/to/model.fvl reachable '"critical" and not "error"' --max-depth 4
cargo run -- proposition always path/to/model.fvl 'not "error"' --max-depth 4
```

### Declarative model file

```text
model "request-grant"
state "idle"
state "waiting"
initial "idle"
edge "idle" "request" "waiting"
edge "waiting" "wait" "waiting"
edge "waiting" "grant" "idle"
label "waiting" "pending"
label "idle" "quiescent"
```

The same finite graph can feed action-temporal, exact-state, proposition, Boolean-proposition, and safety frontends without recompiling Rust.

## CLI exit status

- `0`: property established, or successful structural analysis;
- `1`: safety invariant violation;
- `2`: malformed CLI/model/file/property/metadata/fairness input;
- `3`: configured bounded or staged analysis is `INCONCLUSIVE`;
- `4`: existential target is `UNREACHABLE`;
- `5`: unexpected terminal/deadlock found;
- `6`: universal eventuality is `VIOLATED`;
- `7`: direct single- or multi-response property is `VIOLATED`;
- `8`: deterministic finite monitor verification is `VIOLATED`;
- `9`: generalized Büchi-style acceptance is `VIOLATED`;
- `10`: typed/textual/declarative action-temporal property is `VIOLATED`;
- `11`: exact-state or proposition state property is `VIOLATED`;
- `12`: declarative Boolean safety assertion is `VIOLATED`.

## Tests and CI

Primary gates:

```bash
cargo fmt --all -- --check
cargo build --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Independent/generated evidence retained by the repository includes:

- M8: all **512** directed three-state SCC graphs;
- M9: **4096** universal-eventuality graph/target cases;
- M10: **4096** single-response cases;
- M11: **38,416** two-class response cases;
- M12: **4096** finite-monitor products;
- M13: **8192** generalized Büchi/finite-policy cases;
- M22: **9600** Boolean proposition truth-table cases;
- M23: **3584** safety graph/subset cases;
- M24: **640 bounded reachability + 640 bounded eventuality** cases;
- M30: exact SCC differential across all **512** directed three-state graphs plus **50,000-node** chain/cycle regressions;
- M31: generated weak-fair SCC admissibility checks plus an **8192-case** exact empty-fairness Büchi differential.

M25–M29 retain product/staged semantic and built-binary regression suites. M32 verifies external fairness routing and validation. M33 verifies product/staged fairness, enablement provenance, conclusive retained fair cycles, stage-qualified cutoff honesty, and generous-limit equivalence. M34 adds weak-fair response backend, typed frontend, product/staged, fixed/textual/declarative, and built-binary regressions while keeping all historical response/Büchi/temporal suites green.

## Trust boundaries and limitations

- Results apply to the finite transition model and its atomic-step assumptions, not automatically to machine code, weak-memory executions, or external distributed systems.
- User transition functions, declarative graph files, labels, predicates, and property expressions must faithfully encode the intended system/property; the checker cannot prove modeling fidelity.
- Declarative input is an explicit finite graph plus named state-proposition metadata, not a symbolic transition language or arbitrary state-variable expression language.
- Boolean state-proposition expressions support named atoms with `not`, `and`, `or`, and grouping; they do not provide arbitrary arithmetic/state-field expressions.
- Explicit-state memory grows with the reachable graph. A `k`-clause response monitor may expand a model state into up to `2^k` pending valuations.
- M24 state-property bounds and M25–M29/M33 staged temporal bounds are deterministic exploration budgets, not wall-clock deadlines or total-memory limits.
- Product-only `--max-product-*` limits run after complete model capture and are not a bound on model capture.
- Weak fairness is **never assumed by default**. It is enabled only through explicit `WeakFairness`/`--weak-fair-action` assumptions.
- Weak fairness means continuously enabled actions cannot be postponed forever. The project does **not** implement strong fairness; an action enabled infinitely often but not continuously enabled is not thereby forced.
- Fairness assumptions remain external to the textual temporal grammar. The grammar is still deliberately limited to `response(...)` and `infinitely-often(...)`.
- M34 supports weak fairness for a single response obligation. Multi-response weak-fair composition is not yet implemented.
- Response obligations remain Boolean pending obligations, not per-request identity queues.
- The action-temporal frontend supports exact action atoms only; it has no wildcard/Boolean action predicate language, nested temporal operators, temporal negation, or arbitrary formula composition.
- `all_infinitely_often` is intentionally an infinite-run-only property; finite terminals are ignored for that form. Response fairness instead uses strict finite-terminal handling so a pending finite terminal remains a violation.
- This is not a full LTL/CTL implementation and does not claim arbitrary formula parsing/compilation. The generalized Büchi layer accepts user-defined deterministic action automata; it does not translate arbitrary LTL into Büchi automata.
- No SAT/SMT, BDDs, symbolic execution, theorem proving, symmetry reduction, disk-backed state storage, parallel exploration, or distributed checking is implemented.
- The sleep-set engine remains experimental and differentially audited, not a standalone trusted POR proof backend.
- Deterministic witnesses require deterministic successor ordering; declarative models preserve input edge ordering to make this explicit.
- No milestone makes a performance claim from CI timing.

## Roadmap

Milestones 1–34 now form a coherent explicit-state stack: safety and bounded honesty -> typed models and independent graph validation -> reachability/deadlock/recurrence -> eventuality and response obligations -> finite monitors and generalized Büchi acceptance -> shared graph/product substrates -> typed/textual/declarative specification frontends -> bounded state properties -> product/staged temporal budgets -> iterative deep-graph SCC traversal -> opt-in exact-action weak fairness -> bounded/staged enablement provenance -> weak-fair single-response integration.

The next high-value slice is **Milestone 35: weak-fair multi-response composition**.

M11's canonical multi-response engine tracks one pending bit per named clause and checks infinite failure **per clause**. M35 should extend that semantics under the existing M31/M33 weak-fair contract without collapsing the clauses into an unsound single “some obligation pending” condition.

Acceptance criteria for M35:

- preserve each clause's independent `trigger_i -> eventually response_i` semantics and identify the violated clause;
- preserve finite-terminal precedence and deterministic witnesses;
- quantify infinite counterexamples only over executions admitted by the explicit exact-action weak-fairness assumptions;
- reuse shared fair recurrent analysis and staged enablement provenance rather than add a second fairness traversal;
- preserve product/model cutoff honesty: unknown fair-action enablement must remain `INCONCLUSIVE`;
- preserve exact historical M11 behavior under an empty fairness set, with differential regression evidence;
- expose a coherent typed/backend surface before any broader textual fairness logic is considered;
- add no strong fairness, fairness-by-default behavior, arbitrary LTL/CTL syntax, wall-clock bound, or performance claim.

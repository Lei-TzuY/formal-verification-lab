# Formal Verification Lab

A serious educational laboratory for building formal-methods machinery from first principles in Rust.

The project is an explicit-state verification lab, not a wrapper around an existing model checker and not production-grade verification software. The emphasis is executable semantics, deterministic witnesses, independent graph oracles, honest resource bounds, and explicit trust boundaries.

## Current capability

The repository now has one coherent explicit-state stack from finite transition-system construction through safety, reachability, recurrence, liveness, single- and multi-response obligations, finite-monitor progress/rejection semantics, generalized Büchi acceptance, textual/declarative frontends, deterministic model/product resource budgets, deep-graph SCC traversal, opt-in exact-action weak fairness, and opt-in exact-action strong fairness. Strong fairness is available for generalized Büchi, single-response/action-temporal, multi-response, and finite-monitor verification across unbounded, product-bounded, and staged APIs. M41 exposes the action-temporal paths through the fixed, textual, and declarative temporal CLI routes; M42 keeps multi-response strong fairness at the typed/backend API layer; M43 adds the strong-fair finite-monitor backend/API layer; and M44 closes that backend-to-CLI gap for the direct `monitor` command.

Historical no-fairness behavior remains the default. Weak or strong fairness is enabled only when explicitly supplied, and fairness constrains infinite executions only. Product and staged cutoffs remain proof-honest: missing prefix edges are never treated as proof that an action is disabled, and unresolved enablement provenance produces `INCONCLUSIVE` rather than a false proof or counterexample. Mixed weak-plus-strong assumptions are not silently combined: the CLI fails closed until an explicit combined-fairness semantics is implemented.

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
| M35 | Weak-fair multi-response composition with independent per-clause acceptance and bounded/staged provenance. |
| M36 | Weak-fair finite-monitor progress semantics across unbounded, product-bounded, and staged analysis. |
| M37 | Direct finite-monitor weak-fairness CLI/reporting integration with bounded/staged cutoff honesty. |
| M38 | Opt-in exact-action strong fairness for unbounded generalized Büchi verification with Streett-style recurrent pruning. |
| M39 | Product-bounded and staged strong-fair Büchi verification with conservative enablement provenance and generated cutoff oracles. |
| M40 | Strong-fair single-response and typed/textual/declarative action-temporal composition across unbounded, product-bounded, and staged APIs. |
| M41 | External `--strong-fair-action` temporal CLI/reporting integration with fail-closed mixed-fairness validation and cutoff provenance. |
| M42 | Strong-fair multi-response composition with per-clause acceptance, finite-terminal preservation, and bounded/staged provenance. |
| M43 | Strong-fair finite-monitor progress semantics with rejecting/finite-terminal precedence and bounded/staged enablement provenance. |
| M44 | Direct strong-fair finite-monitor CLI/reporting integration across unbounded, product-bounded, and staged analysis. |

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

### M35 — weak-fair multi-response composition

M35 extends the M11 multi-clause response contract under the same M31/M33 weak-fair execution filter without replacing the canonical pending-bit semantics.

Each response clause retains its own Boolean pending bit. The fair adapter compiles the vector into a deterministic generalized Büchi automaton with **one acceptance set per clause**: clause `i` accepts exactly when `pending[i]` is false. This is deliberately not collapsed into one “some obligation discharged” condition, because that would allow different clauses to alternate and mask starvation of one specific obligation.

`FiniteRunPolicy::RequireAcceptingTerminal` preserves finite pending-terminal violations. For infinite executions, only weakly fair acceptance-avoiding lassos are counterexamples. Returned evidence maps the Büchi acceptance-set identity back to the exact violated response clause and preserves the full pending vector in each trace state.

`check_multi_response_with_weak_fairness`, `check_multi_response_with_weak_fairness_and_product_limits`, and `check_multi_response_with_weak_fairness_and_limits` reuse the existing fair Büchi and staged enablement-provenance engines. Empty fairness delegates exactly to historical M11 paths. Unknown fair-action enablement under a cutoff remains `INCONCLUSIVE`; a real finite terminal or justified fair recurrent violation remains conclusive.

Executable regressions verify that fairness on class B removes only the class-B unfair lasso, unrelated fairness does not discharge that clause, finite pending terminals remain violations, actually taken fair actions do not hide genuine pending cycles, product/model cutoffs remain honest, and generous limits preserve unbounded fair results/accounting. The existing M11 **38,416-case** oracle remains unchanged as the no-fairness compatibility gate.

M35 exposes a typed/backend Rust API; it does not add multi-clause syntax to the textual temporal grammar or claim direct multi-response fairness CLI support.

### M36 — weak-fair finite-monitor progress semantics

M36 composes M12 finite-monitor rejection/progress semantics with the existing M31/M33 weak-fair recurrent machinery instead of introducing a second fairness traversal.

`check_monitor_with_weak_fairness`, `check_monitor_with_weak_fairness_and_product_limits`, and `check_monitor_with_weak_fairness_and_limits` preserve the monitor's precedence tiers exactly:

- a reachable rejecting monitor state remains an immediate violation regardless of fairness;
- a justified finite terminal while a progress condition is active remains a violation because weak fairness constrains only infinite executions;
- only infinite active progress-cycle counterexamples are filtered through exact-action weak fairness.

Progress conditions stay independent. Fairness may eliminate an unfair lasso for one active region without hiding a distinct region that still has a weakly fair recurrent counterexample. The empty fairness set delegates exactly to the historical monitor APIs.

M36 also extracts model-side action-enablement projection into `fair_enablement.rs`. Complete model capture projects exact enablement into product-state ids, while staged capture preserves M33's conservative provenance rule: if a model cutoff leaves fair-action enablement unknown, every configured fair action remains conservatively possible. Missing prefix edges therefore cannot be misused as proof that an action is disabled.

Executable regressions cover matching/unrelated/taken fairness, rejecting-state precedence, finite active terminals, independent progress regions, exact empty-fairness compatibility, product cutoff honesty, staged model-cutoff provenance, and unbounded/staged result-and-evidence equivalence. The full historical M12/M26/M29/M31–M35 suites remain unchanged compatibility gates.

M36 exposes backend Rust APIs only; M37 adds the explicit direct-CLI assumption/reporting path while preserving the same semantics.

### M37 — weak-fair finite-monitor CLI and reporting integration

M37 closes the M36 backend-to-CLI gap without adding another verification engine. The direct `monitor` command accepts repeated `--weak-fair-action <ACTION>` declarations and reuses the same validated `WeakFairness` contract and model/product limit parser already used by temporal verification.

When fairness is supplied, unbounded, product-bounded, and staged monitor invocations route directly into the M36 weak-fair APIs. When fairness is absent, the historical no-fairness monitor path and report remain unchanged. Fairness assumptions are appended explicitly and separately from canonical monitor evidence.

The executable session models distinguish an unfair lasso in which `close` stays continuously enabled from a true finite active terminal. Weak fairness on `close` eliminates only the unfair progress cycle; it cannot excuse rejecting states or finite progress-terminal violations. Product/model cutoffs remain `INCONCLUSIVE` with exit `3`, while conclusive monitor violations retain exit `8`. Duplicate, empty, and missing fairness arguments fail closed with exit `2`.

Built-binary regressions cover fair-lasso elimination, unrelated fairness, rejecting-state/finite-terminal precedence, product and model cutoff honesty, and deterministic fairness-input validation. M37 adds no strong fairness, fairness-by-default behavior, textual monitor language, second CLI analysis engine, wall-clock bound, or performance claim.

### M38 — exact-action strong fairness for generalized Büchi

M38 adds a separate, opt-in strong-fairness execution filter rather than changing `WeakFairness` or the historical no-fairness semantics. `StrongFairness` stores an ordered, validated set of exact action labels. For every configured action `a`, an admitted infinite execution must take `a` infinitely often whenever `a` is enabled infinitely often; intermittent recurring enablement therefore creates an obligation even when weak fairness would not.

`check_buchi_with_strong_fairness` reuses complete model capture, the shared deterministic action-product substrate, acceptance-avoiding residual graphs, and iterative SCC analysis. Strong-fair recurrent admissibility is treated as a Streett-style condition. If a candidate cyclic SCC contains states where a configured action is enabled in the **full product** but contains no internal edge taking that action, those enabled states are removed and cyclic SCC decomposition is repeated so a smaller admissible recurrent subcycle may survive.

Finite terminal policy is unchanged because fairness constrains only infinite executions. Infinite counterexamples remain acceptance-avoiding lassos with deterministic shortest global stems. Their closed recurrent walks explicitly traverse an internal edge for each strong-fair action that is enabled in the repeated component. The empty strong-fairness set delegates exactly to historical `check_buchi`.

Executable evidence includes focused weak-versus-strong intermittent-enablement and recurrent-pruning regressions, an independent **4096-case** two-state graph/action oracle for strong-fair recurrent existence, and an exact **8192-case** empty-strong-fairness differential against the existing M13 Büchi engine. M38 adds no bounded/staged strong-fairness semantics, strong-fair CLI/frontend syntax, fairness-by-default behavior, arbitrary scheduler predicates, wall-clock bound, or performance claim.

### M39 — bounded and staged strong fairness

M39 composes M38's Streett-style recurrent admissibility with the existing deterministic product and staged model/product resource budgets. `check_buchi_with_strong_fairness_and_product_limits` performs complete model capture before bounding product construction, while `check_buchi_with_strong_fairness_and_limits` preserves independent model/product stages and their exact cutoff reasons.

Strong-fair enablement is never inferred from an acceptance-avoiding or truncated edge set. Complete model capture projects authoritative action enablement into retained product-state ids. Staged capture reuses the M33/M36 provenance rule: if a model-state successor relation is not known complete, every configured strong-fair action remains conservatively possible. A missing prefix edge therefore cannot become evidence that an intermittently enabled fairness obligation disappeared.

A retained infinite violation is conclusive only when the acceptance-avoiding recurrent component has real retained edges supporting every required strong-fair enabled/taken pair. Otherwise an incomplete model/product stage remains `INCONCLUSIVE`. Real finite terminal violations keep M38 precedence because fairness still constrains only infinite executions. Empty strong fairness delegates exactly to the historical bounded/staged Büchi APIs, and generous limits preserve the unbounded M38 result, accounting, and evidence contracts.

Executable evidence adds focused product/model cutoff-before/after-witness, finite-terminal, empty-fairness, stage/reason, and generous-limit regressions plus an independent **20,480-case** oracle: all 16 directed two-state graphs × 256 exact-action assignments × 5 product transition limits. The oracle independently reconstructs the retained BFS prefix, evaluates strong-fair recurrent subsets against complete-model enablement, validates `VIOLATED` / `INCONCLUSIVE` / `SATISFIED` classification and accounting, repeats each result for determinism, and verifies that reported cycles use only real retained edges. M39 adds no strong-fair CLI/frontend syntax, fairness-by-default behavior, arbitrary scheduler predicates, wall-clock bounds, or performance claim.

### M40 — strong-fair single-response and action-temporal composition

M40 composes the existing single-response contract with M38/M39 strong-fair Büchi verification rather than introducing a response-specific recurrent traversal. The Boolean pending response obligation is compiled to the established deterministic Büchi form with `FiniteRunPolicy::RequireAcceptingTerminal`, so an unanswered request at a real finite terminal remains a violation even when strong fairness is enabled.

`check_response_with_strong_fairness`, `check_response_with_strong_fairness_and_product_limits`, and `check_response_with_strong_fairness_and_limits` preserve the historical response result, witness, accounting, and stage-qualified cutoff surfaces. Empty strong fairness delegates exactly to the no-fair response APIs. Product/model cutoffs retain conservative action-enablement provenance, so a truncated prefix cannot make an intermittently enabled fair response look disabled.

The typed action-temporal frontend exposes the same unbounded/product-bounded/staged strong-fair paths for both response specifications and recurring-action specifications. Response results normalize back to `TemporalBackend::Response`; recurring-action properties continue to use `TemporalBackend::Buchi`. Textual and declarative adapters reuse the same typed specs and do not extend the temporal grammar.

Executable regressions distinguish strong from weak fairness on intermittently enabled response actions, preserve actually-taken fair-action and finite-terminal violations, check exact empty-fairness compatibility, propagate product/model cutoff provenance, compare generous limits with unbounded results, and exercise real declarative model files. M40 adds no multi-response or finite-monitor strong-fair semantics, mixed-fairness semantics, or CLI assumption syntax.

### M41 — strong-fair temporal CLI and reporting integration

M41 exposes the sealed M40 temporal APIs through repeated `--strong-fair-action <ACTION>` declarations on fixed temporal models, textual `temporal check`, and declarative `temporal file` routes. Unbounded, product-bounded, and staged invocations dispatch to the corresponding M40 backend without changing the property grammar or adding another traversal engine.

Strong-fair assumptions are rendered explicitly and separately from the canonical temporal report. Existing product and staged `INCONCLUSIVE` reason strings and model/product stage provenance remain unchanged. Conclusive temporal violations retain exit `10`; bounded or staged incompleteness remains exit `3`; malformed fairness input remains exit `2`.

No-option and weak-fair temporal invocations retain their historical behavior. Duplicate/empty/missing strong-fair declarations fail closed, and weak plus strong fairness cannot be combined until a deliberate combined-fairness semantics exists. At M41 the direct `monitor` CLI rejected strong fairness rather than silently falling back to no fairness because the strong-fair monitor backend had not yet been implemented; M43 later closes that backend gap while leaving CLI routing for M44.

Built-binary regressions cover fixed/textual/declarative strong-fair routing, unrelated fairness, product/model cutoff provenance, exact report formatting, duplicate/missing/mixed assumption validation, and monitor rejection. M41 adds no multi-response/monitor strong-fair backend, fairness-by-default behavior, wall-clock bound, or performance claim.

### M42 — strong-fair multi-response composition

M42 composes M35's per-clause pending-vector response semantics with the sealed M38/M39 exact-action strong-fair Büchi engines instead of introducing a multi-response-specific recurrent traversal.

Each response clause remains an independent generalized Büchi acceptance set: clause `i` accepts exactly when `pending[i]` is false. `check_multi_response_with_strong_fairness`, `check_multi_response_with_strong_fairness_and_product_limits`, and `check_multi_response_with_strong_fairness_and_limits` preserve the existing multi-response result and evidence surfaces while filtering only infinite executions through strong fairness. `FiniteRunPolicy::RequireAcceptingTerminal` keeps every real finite pending terminal a violation regardless of fairness.

Product-bounded and staged paths inherit M39's conservative exact-action enablement provenance: missing prefix edges cannot prove a strong-fair action disabled, and unresolved work remains `INCONCLUSIVE`. Empty strong fairness delegates exactly to historical no-fair M11/M25/M28 paths, preserving the unchanged **38,416-case** multi-response oracle as a compatibility gate.

Focused executable regressions distinguish intermittent strong versus weak fairness, preserve the exact violated clause under unrelated fairness, keep actually taken fair-action pending cycles and finite pending terminals as real violations, verify product/model cutoff honesty, and compare generous bounded/staged results with the unbounded strong-fair result. M42 adds no direct multi-response strong-fair CLI syntax, strong-fair finite-monitor semantics, mixed weak-plus-strong fairness, fairness-by-default behavior, second traversal engine, wall-clock bound, or performance claim.

### M43 — strong-fair finite-monitor progress semantics

M43 composes M12/M36 finite-monitor rejection/progress semantics with the sealed M38/M39 exact-action strong-fair recurrent machinery instead of adding a monitor-specific strong-fair SCC traversal.

`check_monitor_with_strong_fairness`, `check_monitor_with_strong_fairness_and_product_limits`, and `check_monitor_with_strong_fairness_and_limits` preserve the established monitor result and accounting surfaces. Reachable rejecting monitor states retain global precedence. A real finite terminal in an active progress region remains a violation because strong fairness constrains only infinite executions. Each progress condition is compiled independently as a generalized Büchi acceptance set where an inactive monitor state is accepting; only recurrent active progress cycles are filtered through strong fairness.

Complete model capture uses authoritative action enablement. Staged model capture reuses the conservative M39 enablement-provenance rule for exactly the configured strong-fair actions, so missing prefix edges cannot prove an intermittently enabled action disabled. An unresolved model/product cutoff therefore remains `INCONCLUSIVE` unless a real rejecting state, finite active terminal, or fully justified strong-fair recurrent violation has already been established. Empty strong fairness delegates exactly to historical M12/M26/M29 monitor APIs.

Focused regressions cover intermittent strong-versus-weak fairness, unrelated and actually taken fair actions, rejecting-state and finite-terminal precedence, independent progress regions, exact empty-fairness compatibility, product/model cutoff honesty, and generous-limit evidence equivalence. A generated composition differential checks all 16 directed two-state graph masks × `3^4 = 81` action assignments for **1,296 unbounded cases**, plus three deterministic product-transition limits for **3,888 bounded cases**. Across all **5,184** cases the strong-fair monitor adapter is compared against the equivalent direct strong-fair Büchi construction for status/outcome, accounting, and normalized finite/cycle evidence.

M43 is a typed/backend API milestone. It adds no direct `monitor --strong-fair-action` routing, mixed weak-plus-strong fairness semantics, fairness-by-default behavior, second traversal engine, wall-clock bound, or performance claim.

### M44 — direct strong-fair finite-monitor CLI and reporting integration

M44 closes the remaining M43 backend-to-CLI gap without adding verification semantics. Repeated `--strong-fair-action <ACTION>` declarations on the direct `monitor` command route to the sealed M43 unbounded, product-bounded, or staged strong-fair monitor APIs according to the already shared model/product limit options.

Strong-fair assumptions are rendered explicitly and separately from canonical monitor evidence. Historical no-fairness and weak-fairness monitor paths remain unchanged. Fairness still constrains only infinite progress cycles: reachable rejecting states and real finite active progress terminals retain their existing precedence and remain violations. Product/model cutoffs retain stage-qualified `INCONCLUSIVE` results and exit `3`; conclusive monitor violations retain exit `8`; malformed, duplicate, missing, empty, or mixed weak-plus-strong fairness input fails closed with exit `2`.

Built-binary regressions exercise strong-fair elimination of the unfair `close` lasso, unrelated and actually taken fair actions, rejecting-state/finite-terminal precedence, exact product/model cutoff reason strings, and fairness-option validation. M44 adds no new fairness semantics, combined weak-plus-strong semantics, fairness-by-default behavior, second traversal engine, wall-clock bound, or performance claim.

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
src/strong_fairness.rs        exact-action strong fairness + Streett-style recurrent pruning
src/fair_enablement.rs        complete/conservative model-action enablement projection
src/bounded_fairness.rs       product-bounded/staged weak-fair Büchi composition
src/bounded_strong_fairness.rs product-bounded/staged strong-fair Büchi composition
src/fairness_report.rs        explicit weak/strong monitor + temporal fairness assumption reports
src/eventuality.rs            universal eventuality over target-cut residuals
src/multi_response.rs         no-fair + weak/strong multi-clause response semantics
src/response.rs               no-fair + weak/strong single-response adapters
src/monitor.rs                unbounded, product-bounded + staged finite-monitor semantics
src/monitor_fairness.rs       weak-fair monitor rejection/progress composition
src/monitor_strong_fairness.rs strong-fair monitor progress composition over sealed Büchi core
src/buchi.rs                  unbounded, product-bounded + staged Büchi semantics
src/temporal.rs               typed response/recurring routing, including weak/strong fairness
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

The original transition relation remains owned by `TransitionSystem` plus canonical exploration. Structural and temporal analyses reuse neutral captured finite labeled graphs and one shared deterministic action-product substrate rather than invoking separate traversal engines. Weak and strong fairness are opt-in recurrent-execution filters; neither alters default exploration semantics.

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
cargo run -- monitor session-unfair-close --weak-fair-action close
cargo run -- monitor session-unfair-close --weak-fair-action close --max-product-transitions 3
cargo run -- monitor session-unfair-close --strong-fair-action close
cargo run -- monitor session-unfair-close --strong-fair-action close --max-model-transitions 2
cargo run -- buchi pulses
cargo run -- buchi pulses-unfair --max-model-transitions 2

cargo run -- temporal request-grant
cargo run -- temporal request-grant-unfair
cargo run -- temporal request-grant-unfair --weak-fair-action grant
cargo run -- temporal request-grant-unfair --weak-fair-action grant --max-product-transitions 2
cargo run -- temporal request-grant-unfair --weak-fair-action grant --max-model-transitions 2
cargo run -- temporal request-grant-unfair --strong-fair-action grant
cargo run -- temporal request-grant-unfair --strong-fair-action grant --max-product-transitions 2
cargo run -- temporal request-grant-unfair --strong-fair-action grant --max-model-transitions 1
cargo run -- temporal pulses-unfair --weak-fair-action pulse-b
cargo run -- temporal check request-grant-unfair 'response("request","grant")' --weak-fair-action grant
cargo run -- temporal check request-grant-unfair 'response("request","grant")' --strong-fair-action grant
cargo run -- temporal check pulses-unfair 'infinitely-often("pulse-a","pulse-b")' --weak-fair-action pulse-b
cargo run -- temporal file path/to/model.fvl 'response("request","grant")' --weak-fair-action grant
cargo run -- temporal file path/to/model.fvl 'response("request","grant")' --strong-fair-action grant

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
- M31: generated weak-fair SCC admissibility checks plus an **8192-case** exact empty-fairness Büchi differential;
- M38: **4096** independent strong-fair recurrent graph/action cases plus an **8192-case** exact empty-strong-fairness Büchi differential;
- M39: **20,480** independent bounded strong-fair product-prefix cases across graph/action/transition-limit combinations;
- M43: **5,184** strong-fair monitor-to-Büchi composition differential cases across unbounded and product-bounded graph/action/limit combinations.

M25–M29 retain product/staged semantic and built-binary regression suites. M32 verifies external fairness routing and validation. M33 verifies product/staged fairness, enablement provenance, conclusive retained fair cycles, stage-qualified cutoff honesty, and generous-limit equivalence. M34 adds weak-fair single-response backend/frontend and built-binary regressions. M35 adds per-clause weak-fair multi-response, finite-terminal, fair-cycle, empty-fairness, product-cutoff, staged-cutoff, and generous-limit regressions while retaining all historical M11/M31–M34 gates. M36 adds weak-fair finite-monitor precedence, independent-progress, product/staged cutoff, enablement-provenance, and empty-fairness compatibility regressions while retaining historical M12/M26/M29/M31–M35 coverage. M37 adds built-binary direct-monitor fairness routing, finite-violation precedence, product/model cutoff, and malformed-assumption regressions while preserving all historical no-fairness monitor gates. M38 adds strong-versus-weak fairness separation, Streett-pruning regressions, independent strong-fair recurrent existence, deterministic witness validation, and exact empty-strong-fairness compatibility. M39 adds product/staged cutoff honesty, conservative enablement provenance, conclusive retained strong-fair witness checks, finite-terminal precedence, empty-fairness/generous-limit compatibility, stage-qualified inconclusive reasons, and the 20,480-case independent prefix oracle. M40 adds strong-fair single-response and typed/textual/declarative temporal differential regressions, including intermittent enablement, finite terminals, empty-fairness compatibility, bounded/staged provenance, generous limits, and real external-file integration. M41 adds built-binary strong-fair temporal CLI routing/reporting and fail-closed malformed/mixed-assumption regressions while retaining all historical no-fair and weak-fair CLI gates. M42 adds per-clause strong-fair multi-response regressions for intermittent enablement, unrelated/taken fairness, finite-terminal precedence, exact empty-fairness compatibility, product/model cutoff honesty, and generous-limit equivalence while retaining the historical 38,416-case M11 oracle and M39 strong-fair cutoff coverage. M43 adds strong-fair finite-monitor precedence, independent-progress, product/staged provenance and generous-limit regressions plus the 5,184-case generated adapter differential against the sealed M38/M39 Büchi engines. M44 adds built-binary direct-monitor strong-fair routing/reporting, lasso-filtering, rejecting/finite-terminal precedence, exact cutoff-provenance, and malformed/mixed-assumption regressions.

## Trust boundaries and limitations

- Results apply to the finite transition model and its atomic-step assumptions, not automatically to machine code, weak-memory executions, or external distributed systems.
- User transition functions, declarative graph files, labels, predicates, and property expressions must faithfully encode the intended system/property; the checker cannot prove modeling fidelity.
- Declarative input is an explicit finite graph plus named state-proposition metadata, not a symbolic transition language or arbitrary state-variable expression language.
- Boolean state-proposition expressions support named atoms with `not`, `and`, `or`, and grouping; they do not provide arbitrary arithmetic/state-field expressions.
- Explicit-state memory grows with the reachable graph. A `k`-clause response monitor may expand a model state into up to `2^k` pending valuations.
- M24 state-property bounds and M25–M29/M33/M39–M43 staged temporal bounds are deterministic exploration budgets, not wall-clock deadlines or total-memory limits.
- Product-only `--max-product-*` limits run after complete model capture and are not a bound on model capture.
- Weak fairness is **never assumed by default**. It is enabled only through explicit `WeakFairness`/`--weak-fair-action` assumptions.
- Weak fairness means continuously enabled actions cannot be postponed forever.
- Strong fairness is also **never assumed by default**. M38–M44 support exact-action strong fairness for generalized Büchi, single-response/action-temporal, typed/backend multi-response, and finite-monitor verification across unbounded, product-bounded, and staged APIs; M41 exposes the action-temporal paths and M44 exposes direct finite-monitor verification through explicit `--strong-fair-action` assumptions.
- Strong fairness means an action enabled infinitely often on an admitted infinite execution must also be taken infinitely often; intermittent enablement therefore matters.
- Fairness assumptions remain external to the textual temporal grammar. The grammar is still deliberately limited to `response(...)` and `infinitely-often(...)`.
- Weak and strong fairness cannot currently be combined in one analysis; M41/M44 CLI routing fails closed on mixed assumptions rather than inventing semantics. A combined profile requires an explicit semantic design rather than dispatch precedence.
- M42 exposes multi-response weak/strong fairness through typed/backend APIs. The direct `respond dual-grant` CLI remains the historical no-fairness surface, and no multi-clause textual temporal syntax is introduced.
- M37 exposes finite-monitor weak fairness on the direct `monitor` CLI, and M44 exposes the sealed M43 strong-fair finite-monitor backend through the same direct command for unbounded, product-bounded, and staged analysis.
- Response obligations remain Boolean pending obligations, not per-request identity queues.
- The action-temporal frontend supports exact action atoms only; it has no wildcard/Boolean action predicate language, nested temporal operators, temporal negation, or arbitrary formula composition.
- `all_infinitely_often` is intentionally an infinite-run-only property; finite terminals are ignored for that form. Response fairness instead uses strict finite-terminal handling so a pending finite terminal remains a violation.
- This is not a full LTL/CTL implementation and does not claim arbitrary formula parsing/compilation. The generalized Büchi layer accepts user-defined deterministic action automata; it does not translate arbitrary LTL into Büchi automata.
- No SAT/SMT, BDDs, symbolic execution, theorem proving, symmetry reduction, disk-backed state storage, parallel exploration, or distributed checking is implemented.
- The sleep-set engine remains experimental and differentially audited, not a standalone trusted POR proof backend.
- Deterministic witnesses require deterministic successor ordering; declarative models preserve input edge ordering to make this explicit.
- No milestone makes a performance claim from CI timing.

## Roadmap

Milestones 1–44 now form a coherent explicit-state stack: safety and bounded honesty -> typed models and independent graph validation -> reachability/deadlock/recurrence -> eventuality and response obligations -> finite monitors and generalized Büchi acceptance -> shared graph/product substrates -> typed/textual/declarative specification frontends -> bounded state properties -> product/staged temporal budgets -> iterative deep-graph SCC traversal -> opt-in exact-action weak fairness -> bounded/staged enablement provenance -> weak-fair response and finite-monitor composition -> direct weak-fair monitor integration -> opt-in exact-action strong fairness -> proof-honest bounded/staged strong-fair Büchi verification -> strong-fair single-response/action-temporal composition -> external strong-fair temporal CLI integration -> per-clause strong-fair multi-response composition -> strong-fair finite-monitor composition -> direct strong-fair monitor CLI/reporting integration.

M44 completes the separate weak-fair and strong-fair verticals. Continuing to add one-off routing variants would now have low architectural value. The next architectural phase is **Milestone 45: explicit combined weak/strong fairness profile at the generalized Büchi/recurrent substrate**.

M45 should define one validated fairness profile containing independently ordered weak- and strong-fair exact-action assumptions and verify infinite executions only when they satisfy **all** configured obligations. It must be a semantic core milestone first, not a CLI shortcut: existing no-fair, weak-only, and strong-only APIs remain compatibility authorities while mixed execution filtering is introduced once at the Büchi/recurrent layer and only then composed upward in later slices.

Acceptance criteria for M45:

- define explicit overlap semantics when the same exact action appears in both sets; because strong fairness implies the corresponding weak obligation, overlap must be canonicalized or otherwise specified deterministically rather than evaluated ambiguously;
- preserve fairness as an infinite-execution filter only, leaving finite-terminal policy unchanged;
- reuse complete-model action enablement and existing SCC/product substrates rather than creating another traversal engine;
- return only real closed recurrent witnesses satisfying every weak and strong obligation simultaneously;
- preserve exact no-fair, weak-only, and strong-only results/evidence through differential compatibility tests;
- include independent small-graph validation that distinguishes continuously enabled weak obligations from intermittently enabled strong obligations and exercises overlapping assumptions;
- do not expose mixed CLI syntax until the combined backend semantics and witness contracts are sealed;
- make no performance claim from CI timing.

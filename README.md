# Formal Verification Lab

A serious educational laboratory for building formal-methods machinery from first principles in Rust.

The project is an explicit-state verification lab, not a wrapper around an existing model checker and not production-grade verification software. The emphasis is executable semantics, deterministic witnesses, independent graph oracles, honest resource bounds, and explicit trust boundaries.

## Current capability

The repository now has one coherent explicit-state stack from finite transition-system construction through safety, reachability, recurrence, liveness, single- and multi-response obligations, finite-monitor progress/rejection semantics, generalized Büchi acceptance, textual/declarative frontends, deterministic model/product resource budgets, deep-graph SCC traversal, opt-in exact-action weak fairness, opt-in exact-action strong fairness, and an explicit combined weak/strong fairness profile. Strong fairness is available for generalized Büchi, single-response/action-temporal, multi-response, and finite-monitor verification across unbounded, product-bounded, and staged APIs. M45 introduced the typed `FairnessProfile`, M46 carries mixed profiles through generalized Büchi product-bounded and staged analysis, M47 propagates the same profile through single-response plus typed/textual/declarative action-temporal APIs, M48 exposes that combined profile on the external temporal CLI, M49 composes the same canonical profile with multi-response verification, M50 composes it with finite-monitor verification, M51 exposes combined-fair multi-response verification on the direct `respond dual-grant*` CLI, and M52 exposes the same canonical weak/strong profile on direct `respond request-grant*` single-response routes while preserving exact no-fair, weak-only, and strong-only compatibility paths.

Historical no-fairness behavior remains the default. Fairness is enabled only when explicitly supplied, and it constrains infinite executions only. Product and staged cutoffs remain proof-honest: missing prefix edges are never treated as proof that an action is disabled, and unresolved enablement provenance produces `INCONCLUSIVE` rather than a false proof or counterexample. M48 accepts simultaneous repeated `--weak-fair-action` and `--strong-fair-action` declarations on fixed temporal, textual `temporal check`, and declarative `temporal file` routes, canonicalizing overlap to the strong class. M49 adds typed/backend mixed-fairness multi-response composition across unbounded, product-bounded, and staged APIs. M50 adds the same combined profile to finite-monitor verification and exposes mixed weak/strong assumptions on the direct `monitor` CLI with canonical overlap handling. M51 exposes the M49 multi-response authority through direct `respond dual-grant*` routes with canonical weak/strong reporting and bounded/staged cutoff provenance. M52 closes the corresponding single-response CLI gap: direct `respond request-grant*` routes now reuse the sealed M47 response fairness adapters, preserve finite pending terminals and historical no-option behavior, and retain the same model-before-product cutoff provenance.

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
| M45 | Explicit combined weak/strong fairness profile for unbounded generalized Büchi verification. |
| M46 | Product-bounded and staged combined weak/strong fairness with proof-honest enablement provenance. |
| M47 | Combined-fair single-response and typed/textual/declarative action-temporal composition across unbounded, product-bounded, and staged APIs. |
| M48 | External combined-fair temporal CLI/reporting across fixed, textual, and declarative routes with bounded/staged cutoff provenance. |
| M49 | Combined-fair multi-response composition with per-clause acceptance, finite-terminal preservation, and bounded/staged provenance. |
| M50 | Combined-fair finite-monitor composition plus direct mixed-fair monitor CLI/reporting with rejecting/finite-terminal precedence and bounded/staged provenance. |
| M51 | External combined-fair multi-response CLI/reporting with canonical overlap, finite pending-terminal evidence, and bounded/staged provenance. |
| M52 | External combined-fair single-response CLI/reporting with canonical overlap, finite pending-terminal evidence, historical no-option compatibility, and bounded/staged provenance. |

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

Finite terminal policy is unchanged because weak fairness constrains only infinite executions. Infinite counterexamples remain acceptance-avoiding lassos with deterministic shortest global stems. Their closed recurrent walks explicitly traverse an internal edge for each strong-fair action that is enabled in the repeated component. The empty strong-fairness set delegates exactly to historical `check_buchi`.

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

### M45 — combined weak/strong fairness core

M45 introduces one validated `FairnessProfile` for unbounded generalized Büchi verification. Weak and strong exact-action assumptions retain independent declaration order. If the same action appears in both classes, the profile canonicalizes it to the strong class because strong fairness subsumes the corresponding weak obligation.

Empty, weak-only, and strong-only profiles delegate exactly to the sealed historical backends. Mixed profiles reuse complete model capture, the shared deterministic action-product graph, acceptance-avoiding residuals, iterative SCC decomposition, and shortest-stem ordering. Strong obligations use the existing Streett-style recurrent pruning rule; every surviving recurrent component must also satisfy every weak obligation.

Finite terminal policy is unchanged because fairness still filters only infinite executions. A mixed infinite counterexample is a real closed recurrent walk that simultaneously witnesses all configured weak and strong obligations. Executable evidence includes focused validation/finite/intermittent-enablement/compatibility regressions, an independent **512-case** two-state graph oracle covering mixed and overlapping assumptions, and an explicit shortest-stem/cycle-entry alignment regression.

M45 is a typed generalized Büchi core milestone. It adds no mixed CLI syntax, no product-bounded or staged mixed-fairness API, no fairness-by-default behavior, no second traversal engine, and no performance claim.

### M46 — proof-honest bounded and staged combined fairness

M46 carries the M45 `FairnessProfile` through deterministic product-only budgets and independent model/product staged budgets without introducing another graph traversal or fairness engine. `check_buchi_with_fairness_profile_and_product_limits` performs complete model capture before bounding the action product, while `check_buchi_with_fairness_profile_and_limits` preserves the existing model-before-product stage contract and exact cutoff reasons.

Empty, weak-only, and strong-only profiles delegate exactly to their sealed historical bounded/staged backends. Mixed product-bounded verification projects authoritative complete-model enablement into the retained product. Mixed staged verification reuses bounded capture provenance: if a retained model state's successor vector is not known complete, every configured weak or strong fair action remains conservatively possible. Missing prefix edges therefore cannot become proof that a weak action is disabled or that a strong action is absent.

Finite-terminal policy keeps global precedence because fairness still filters only infinite executions. A retained mixed infinite violation is conclusive only when real retained product edges form a closed recurrent walk that simultaneously satisfies every configured weak and strong obligation. If no such witness has been established, incomplete model work takes precedence over incomplete product work and returns stage-qualified `INCONCLUSIVE`; complete analysis without a counterexample returns `SATISFIED`.

Executable evidence preserves exact no-fair/weak-only/strong-only delegation, generous-limit equality with M45, strict finite-terminal precedence, and product-stage reason reporting. An independent generated oracle covers all `4^4 = 256` labeled two-state edge assignments, both mixed and overlap-canonicalized profiles, and five deterministic transition limits on both product-only and staged paths: **5,120 bounded/staged cases**. It independently reconstructs retained BFS prefixes, conservative enablement, recurrent subsets, mixed fairness admissibility, accounting, cutoff classification, and real closed-walk witness edges.

M46 seals combined fairness at the generalized Büchi core across unbounded, product-bounded, and staged analysis. Higher-level composition is deliberately layered on top rather than duplicated in the core.

### M47 — combined-fair response and action-temporal composition

M47 propagates the sealed M45/M46 `FairnessProfile` through the existing single-response adapter and typed action-temporal frontend without adding another recurrent traversal. Empty, weak-only, and strong-only profiles delegate exactly to their historical response/temporal paths; only genuinely mixed profiles enter the combined-fair Büchi engines.

Single-response obligations retain the canonical deterministic pending-bit automaton and `FiniteRunPolicy::RequireAcceptingTerminal`, so a real finite terminal with a pending request remains a violation regardless of fairness. `all_infinitely_often` retains its existing infinite-run-only policy, so finite terminals remain ignored for that specification. Product-bounded and staged variants preserve M46's authoritative/conservative enablement provenance and exact model-before-product `INCONCLUSIVE` reasons.

The response and temporal APIs preserve existing result types, backend identity, accounting, witness normalization, and declarative/textual parsing surfaces. M47 does not add temporal grammar or mixed CLI dispatch. Executable evidence differentially compares mixed response and recurring-action results with direct M45/M46 Büchi construction, checks exact no-fair/weak-only/strong-only and overlap-canonicalized compatibility, finite-terminal precedence, cutoff-before/after-witness behavior, generous-limit equality, real closed lassos, and external declarative model-file integration.

M47 seals mixed fairness through typed/frontend response and action-temporal composition. Mixed temporal CLI/reporting, multi-response composition, and finite-monitor composition remain deliberate later frontiers.

### M48 — external combined-fair temporal CLI and reporting integration

M48 promotes the M47 combined-fair temporal APIs to the external command surface without adding another verification engine. Fixed temporal models, textual `temporal check`, and declarative `temporal file` routes accept simultaneous repeated `--weak-fair-action <ACTION>` and `--strong-fair-action <ACTION>` declarations.

CLI parsing still validates each class independently, then constructs the canonical M45 `FairnessProfile`; an action declared in both classes is reported and verified only as strong. Empty/no-fair, weak-only, and strong-only invocations retain their historical dispatch paths. Genuinely mixed profiles route through the M47 unbounded, product-bounded, or staged APIs according to the existing model/product limit options.

Weak and strong assumptions are rendered explicitly and separately from canonical temporal evidence. Exit `0` means satisfaction, `10` a conclusive temporal violation, `3` bounded/staged `INCONCLUSIVE`, and `2` malformed fairness/options. Product/model cutoff reason and stage provenance remain unchanged. Built-binary regressions cover distinct mixed assumptions, overlap canonicalization, real mixed-profile violations, fixed/textual/declarative routing, product/model cutoffs, malformed input, and continued fail-closed mixed monitor routing.

M48 adds no temporal grammar, fairness-by-default behavior, mixed multi-response or finite-monitor semantics, second traversal engine, wall-clock proof bound, or performance claim.

### M49 — combined-fair multi-response composition

M49 propagates the canonical M45 `FairnessProfile` through the existing multi-clause response adapter without adding a multi-response-specific fairness traversal. `check_multi_response_with_fairness_profile`, `check_multi_response_with_fairness_profile_and_product_limits`, and `check_multi_response_with_fairness_profile_and_limits` preserve the established multi-response result, accounting, cutoff, and evidence surfaces.

Each response clause remains one independent generalized Büchi acceptance set where clause `i` accepts exactly when `pending[i]` is false. The adapter therefore cannot let different pending classes alternate and hide starvation of one clause. `FiniteRunPolicy::RequireAcceptingTerminal` remains authoritative, so a real finite terminal with any pending obligation is a violation even under a mixed fairness profile. Counterexamples preserve the exact violated clause and full pending vector.

Empty, weak-only, and strong-only profiles delegate exactly to the sealed no-fair, M35 weak-fair, and M42 strong-fair authorities. Only genuinely mixed profiles enter the M45/M46 combined-fair Büchi engines. Product-only analysis therefore retains complete-model action enablement, while staged analysis retains conservative model-prefix enablement provenance and model-before-product `INCONCLUSIVE` precedence.

Executable evidence includes focused mixed filtering, finite-terminal, retained closed-lasso, cutoff-before/after-witness, stage-provenance, and generous-limit regressions. A generated adapter differential covers all 16 directed two-state graphs × `4^4 = 256` response/fairness action assignments: **4,096 unbounded**, **12,288 product-bounded**, and **12,288 staged** cases. Across all **28,672** cases, the M49 adapter is compared directly with the equivalent M45/M46 generalized Büchi authority for outcome/status, model/product accounting, exact clause identity, pending-vector traces, and finite/lasso evidence.

M49 is a typed/backend composition milestone. It adds no direct mixed-fair multi-response CLI syntax, no multi-clause textual temporal grammar, no fairness-by-default behavior, no second traversal engine, no wall-clock proof bound, and no performance claim.

### M50 — combined-fair finite-monitor composition and CLI

M50 propagates the canonical M45 `FairnessProfile` through finite-monitor verification across unbounded, product-bounded, and staged APIs. Empty, weak-only, and strong-only profiles delegate exactly to the sealed no-fair, M36 weak-fair, and M43 strong-fair monitor authorities; only genuinely mixed profiles enter the M45/M46 combined-fair generalized Büchi recurrent engine.

Monitor precedence is unchanged: reachable rejecting states remain globally first, real finite terminals with an active progress condition remain violations because fairness constrains only infinite executions, and only infinite active progress cycles are filtered by fairness. Independent progress-condition identity, monitor-state evidence, complete-model enablement, conservative staged provenance, deterministic model-before-product cutoff precedence, and `INCONCLUSIVE` honesty are preserved.

The direct `monitor` command now accepts simultaneous repeated `--weak-fair-action` and `--strong-fair-action` declarations. `FairnessProfile` canonicalizes overlap into the strong class, reports both canonical classes explicitly, and routes mixed unbounded/product/staged invocations into the M50 backend while retaining historical no/weak/strong paths. Monitor violations retain exit `8`; unresolved resource cutoffs retain exit `3`; malformed or duplicate assumptions retain exit `2`.

Executable evidence includes focused mixed-filtering, rejecting/finite-terminal precedence, exact compatibility delegation, product/model cutoff provenance, and generous-limit regressions; generated adapter differentials cover **4,096 unbounded**, **12,288 product-bounded**, and **20,480 staged** cases (**36,864 total**) against the sealed combined-fair Büchi authority. Built-binary regressions additionally verify mixed assumption routing, overlap canonicalization, explicit reporting, finite/rejecting precedence, and product/model cutoff behavior.

M50 adds no fairness-by-default behavior, textual monitor language, second traversal engine, wall-clock proof bound, or performance claim.

### M51 — external combined-fair multi-response CLI/reporting

M51 exposes the sealed M49 multi-response fairness-profile adapters on the direct `respond dual-grant*` command surface without adding another verification engine. The direct multi-response routes accept repeated `--weak-fair-action` and `--strong-fair-action` declarations, construct the canonical M45 `FairnessProfile`, and route unbounded, product-bounded, or staged requests into the corresponding M49 authority.

No-option multi-response invocations preserve the historical M11/M25/M28 paths exactly. Fairness constrains only infinite executions: a real finite terminal with a pending clause remains a violation. `dual-grant-terminal-b` is an executable teaching model for that boundary. Overlapping weak/strong declarations are canonicalized into the strong class, and reports append both canonical assumption classes separately from unchanged multi-response evidence.

Product-only and staged routes retain existing accounting and proof-honest cutoff semantics. Unresolved work returns exit `3`; conclusive multi-response violations retain exit `7`; malformed or duplicate fairness input returns exit `2`. Built-binary regressions cover mixed routing, overlap canonicalization, finite pending terminals, product/model cutoffs, malformed assumptions, explicit usage, and historical no-option compatibility. The M49 **28,672-case** differential remains the semantic authority underneath this external wiring.

M51 adds no multi-clause textual temporal grammar, fairness-by-default behavior, second traversal/fairness representation, wall-clock proof bound, or performance claim.

### M52 — external combined-fair single-response CLI/reporting

M52 closes the direct single-response integration gap without adding response semantics or traversal. The `respond request-grant*` routes now accept repeated `--weak-fair-action` and `--strong-fair-action` declarations, build the canonical M45 `FairnessProfile`, and dispatch unbounded, product-bounded, or staged requests to the sealed M47 single-response adapters.

No-option invocations keep the historical M10/M25/M28 path and report exactly. Fairness still constrains only infinite executions, so `request-grant-terminal` provides a real finite pending-terminal boundary that remains a violation regardless of weak, strong, or mixed assumptions. Overlap is canonicalized to the strong class, and fairness reporting is appended separately from canonical single-response status, accounting, and finite/lasso evidence.

Product-only and staged routes preserve existing proof-honest cutoff semantics and deterministic model-before-product provenance. Unresolved work returns exit `3`; conclusive direct response violations retain exit `7`; malformed, duplicate, empty, or missing fairness input returns exit `2`. Built-binary regressions cover weak-only, strong-only, genuinely mixed routing, overlap canonicalization, real pending lassos, finite pending terminals, product cutoff before/after a witness, staged model-before-product cutoff provenance, malformed/unknown options, usage text, and historical no-option compatibility.

M52 adds no fairness-by-default behavior, new response semantics, new traversal engine, textual fairness grammar, wall-clock proof bound, or performance claim.

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
src/combined_fairness.rs      combined weak/strong fairness profile + recurrent witnesses
src/fair_enablement.rs        complete/conservative model-action enablement projection
src/bounded_fairness.rs       product-bounded/staged weak-fair Büchi composition
src/bounded_strong_fairness.rs product-bounded/staged strong-fair Büchi composition
src/bounded_combined_fairness.rs product-bounded/staged combined-fair Büchi composition
src/fairness_report.rs        explicit weak/strong monitor + temporal fairness assumption reports
src/eventuality.rs            universal eventuality over target-cut residuals
src/multi_response.rs         no-fair + weak/strong/combined multi-clause response semantics
src/response.rs               no-fair + weak/strong/combined single-response adapters
src/monitor.rs                unbounded, product-bounded + staged finite-monitor semantics
src/monitor_fairness.rs       weak-fair monitor rejection/progress composition
src/monitor_strong_fairness.rs strong-fair monitor progress composition over sealed Büchi core
src/monitor_combined_fairness.rs combined weak/strong monitor composition over sealed Büchi core
src/buchi.rs                  unbounded, product-bounded + staged Büchi semantics
src/temporal.rs               typed response/recurring routing, including weak/strong/combined fairness
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

The original transition relation remains owned by `TransitionSystem` plus canonical exploration. Structural and temporal analyses reuse neutral captured finite labeled graphs and one shared deterministic action-product substrate rather than invoking separate traversal engines. Weak, strong, and combined fairness are opt-in recurrent-execution filters; none alters default exploration semantics.

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
cargo run -- respond request-grant-unfair --weak-fair-action grant
cargo run -- respond request-grant-unfair --weak-fair-action unrelated --strong-fair-action grant
cargo run -- respond request-grant-terminal --strong-fair-action grant
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
cargo run -- temporal request-grant-unfair --weak-fair-action unrelated --strong-fair-action grant
cargo run -- temporal pulses-unfair --weak-fair-action pulse-b
cargo run -- temporal check request-grant-unfair 'response("request","grant")' --weak-fair-action grant
cargo run -- temporal check request-grant-unfair 'response("request","grant")' --strong-fair-action grant
cargo run -- temporal check request-grant-unfair 'response("request","grant")' --weak-fair-action unrelated --strong-fair-action grant
cargo run -- temporal check pulses-unfair 'infinitely-often("pulse-a","pulse-b")' --weak-fair-action pulse-b
cargo run -- temporal file path/to/model.fvl 'response("request","grant")' --weak-fair-action grant
cargo run -- temporal file path/to/model.fvl 'response("request","grant")' --strong-fair-action grant
cargo run -- temporal file path/to/model.fvl 'response("request","grant")' --weak-fair-action unrelated --strong-fair-action grant

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
- M43: **5,184** strong-fair monitor-to-Büchi composition differential cases across unbounded and product-bounded graph/action/limit combinations;
- M45: **512** independent two-state mixed/overlap fairness cases plus focused compatibility and lasso-alignment regressions;
- M46: **5,120** independent bounded/staged mixed-or-overlap fairness cases across labeled two-state graphs and deterministic transition limits;
- M49: **28,672** combined-fair multi-response adapter differential cases: 4,096 unbounded + 12,288 product-bounded + 12,288 staged;
- M50: **36,864** combined-fair finite-monitor adapter differential cases: 4,096 unbounded + 12,288 product-bounded + 20,480 staged.

M25–M29 retain product/staged semantic and built-binary regression suites. M32 verifies external fairness routing and validation. M33 verifies product/staged fairness, enablement provenance, conclusive retained fair cycles, stage-qualified cutoff honesty, and generous-limit equivalence. M34 adds weak-fair single-response backend/frontend and built-binary regressions. M35 adds per-clause weak-fair multi-response, finite-terminal, fair-cycle, empty-fairness, product-cutoff, staged-cutoff, and generous-limit regressions while retaining all historical M11/M31–M34 gates. M36 adds weak-fair finite-monitor precedence, independent-progress, product/staged cutoff, enablement-provenance, and empty-fairness compatibility regressions while retaining historical M12/M26/M29/M31–M35 coverage. M37 adds built-binary direct-monitor fairness routing, finite-violation precedence, product/model cutoff, and malformed-assumption regressions while preserving all historical no-fairness monitor gates. M38 adds strong-versus-weak fairness separation, Streett-pruning regressions, independent strong-fair recurrent existence, deterministic witness validation, and exact empty-strong-fairness compatibility. M39 adds product/staged cutoff honesty, conservative enablement provenance, conclusive retained strong-fair witness checks, finite-terminal precedence, empty-fairness/generous-limit compatibility, stage-qualified inconclusive reasons, and the 20,480-case independent prefix oracle. M40 adds strong-fair single-response and typed/textual/declarative temporal differential regressions, including intermittent enablement, finite terminals, empty-fairness compatibility, bounded/staged provenance, generous limits, and real external-file integration. M41 adds built-binary strong-fair temporal CLI routing/reporting and fail-closed malformed/mixed-assumption regressions while retaining all historical no-fair and weak-fair CLI gates. M42 adds per-clause strong-fair multi-response regressions for intermittent enablement, unrelated/taken fairness, finite-terminal precedence, exact empty-fairness compatibility, product/model cutoff honesty, and generous-limit equivalence while retaining the historical 38,416-case M11 oracle and M39 strong-fair cutoff coverage. M43 adds strong-fair finite-monitor precedence, independent-progress, product/staged provenance and generous-limit regressions plus the 5,184-case generated adapter differential against the sealed M38/M39 Büchi engines. M44 adds built-binary direct-monitor strong-fair routing/reporting, lasso-filtering, rejecting/finite-terminal precedence, exact cutoff-provenance, and malformed/mixed-assumption regressions. M45 adds mixed weak/strong profile validation, overlap canonicalization, finite-policy preservation, exact single-class compatibility, one-walk witness validation, the 512-case independent recurrent oracle, and shortest-stem/cycle-entry alignment coverage. M46 adds exact bounded/staged no-fair/weak-only/strong-only delegation, generous-limit evidence equality, finite-terminal precedence, conservative mixed enablement provenance, exact model/product cutoff accounting and stage classification, plus the 5,120-case independent prefix/recurrent/fairness oracle. M47 adds ten direct composition regressions covering exact compatibility delegation, mixed response/recurring differential equality against M45/M46, finite-terminal preservation, cutoff-before/after-witness behavior, stage provenance, normalized closed lassos, and real textual/declarative external-model integration. M48 adds built-binary combined-fair CLI regressions for distinct and overlapping profiles, preserved real violations, fixed/textual/declarative routing, product/model cutoff provenance, malformed assumptions, and continued fail-closed mixed monitor routing. M49 adds exact no/weak/strong delegation across unbounded/product/staged paths, focused genuinely mixed filtering/finite/lasso/cutoff regressions, and the 28,672-case adapter differential against the sealed combined-fair Büchi authority. M50 adds exact no/weak/strong monitor delegation, mixed progress filtering, rejecting/finite-terminal precedence, product/staged provenance, the 36,864-case monitor adapter differential, and built-binary mixed monitor routing/reporting regressions. M51 adds built-binary direct multi-response mixed-fair routing/reporting, overlap canonicalization, finite pending-terminal, bounded/staged provenance, malformed-assumption, usage, and historical no-option regressions. M52 adds the corresponding direct single-response built-binary regressions across weak/strong/mixed routing, canonical overlap, retained real lassos, finite pending terminals, product cutoff before/after witness, staged model-before-product provenance, malformed/unknown options, usage, and no-option compatibility.

## Trust boundaries and limitations

- Results apply to the finite transition model and its atomic-step assumptions, not automatically to machine code, weak-memory executions, or external distributed systems.
- User transition functions, declarative graph files, labels, predicates, and property expressions must faithfully encode the intended system/property; the checker cannot prove modeling fidelity.
- Declarative input is an explicit finite graph plus named state-proposition metadata, not a symbolic transition language or arbitrary state-variable expression language.
- Boolean state-proposition expressions support named atoms with `not`, `and`, `or`, and grouping; they do not provide arbitrary arithmetic/state-field expressions.
- Explicit-state memory grows with the reachable graph. A `k`-clause response monitor may expand a model state into up to `2^k` pending valuations.
- M24 state-property bounds and M25–M29/M33/M39–M52 staged temporal bounds are deterministic exploration budgets, not wall-clock deadlines or total-memory limits.
- Product-only `--max-product-*` limits run after complete model capture and are not a bound on model capture.
- Weak fairness is **never assumed by default**. It is enabled only through explicit `WeakFairness`/`--weak-fair-action` assumptions.
- Weak fairness means continuously enabled actions cannot be postponed forever.
- Strong fairness is also **never assumed by default**. M38–M44 support exact-action strong fairness for generalized Büchi, single-response/action-temporal, typed/backend multi-response, and finite-monitor verification across unbounded, product-bounded, and staged APIs; M41 exposes the action-temporal paths and M44 exposes direct finite-monitor verification through explicit `--strong-fair-action` assumptions.
- Strong fairness means an action enabled infinitely often on an admitted infinite execution must also be taken infinitely often; intermittent enablement therefore matters.
- Fairness assumptions remain external to the textual temporal grammar. The grammar is still deliberately limited to `response(...)` and `infinitely-often(...)`.
- M45–M52 provide explicit combined weak/strong fairness semantics through generalized Büchi, single-response, typed/textual/declarative action-temporal, multi-response, and finite-monitor APIs across unbounded, product-bounded, and staged analysis; M48 exposes mixed temporal assumptions on the CLI, M50 exposes mixed finite-monitor assumptions on `monitor`, M51 exposes mixed multi-response assumptions on direct `respond dual-grant*` routes, and M52 exposes mixed single-response assumptions on direct `respond request-grant*` routes.
- M35/M42/M49 provide the multi-response weak/strong/combined fairness backend authority, and M51 exposes it externally for the direct multi-response teaching models. No multi-clause textual temporal syntax is introduced yet.
- M37 exposes finite-monitor weak fairness on the direct `monitor` CLI, M44 exposes strong-fair monitor verification, and M50 exposes canonical mixed weak/strong monitor assumptions through the same direct command for unbounded, product-bounded, and staged analysis.
- Response obligations remain Boolean pending obligations, not per-request identity queues.
- The action-temporal frontend supports exact action atoms only; it has no wildcard/Boolean action predicate language, nested temporal operators, temporal negation, or arbitrary formula composition.
- `all_infinitely_often` is intentionally an infinite-run-only property; finite terminals are ignored for that form. Response fairness instead uses strict finite-terminal handling so a pending finite terminal remains a violation.
- This is not a full LTL/CTL implementation and does not claim arbitrary formula parsing/compilation. The generalized Büchi layer accepts user-defined deterministic action automata; it does not translate arbitrary LTL into Büchi automata.
- No SAT/SMT, BDDs, symbolic execution, theorem proving, symmetry reduction, disk-backed state storage, parallel exploration, or distributed checking is implemented.
- The sleep-set engine remains experimental and differentially audited, not a standalone trusted POR proof backend.
- Deterministic witnesses require deterministic successor ordering; declarative models preserve input edge ordering to make this explicit.
- No milestone makes a performance claim from CI timing.

## Roadmap

Milestones 1–52 now form a coherent explicit-state stack: safety and bounded honesty -> typed models and independent graph validation -> reachability/deadlock/recurrence -> eventuality and response obligations -> finite monitors and generalized Büchi acceptance -> shared graph/product substrates -> typed/textual/declarative specification frontends -> bounded state properties -> product/staged temporal budgets -> iterative deep-graph SCC traversal -> opt-in exact-action weak fairness -> bounded/staged enablement provenance -> weak-fair response and monitor composition -> opt-in exact-action strong fairness -> proof-honest bounded/staged strong-fair verification -> strong-fair response/temporal/multi-response/monitor composition -> explicit combined weak/strong fairness -> proof-honest bounded/staged combined-fair Büchi -> combined-fair response/action-temporal -> external combined-fair temporal CLI -> combined-fair multi-response -> combined-fair finite-monitor backend/direct CLI -> external combined-fair multi-response CLI/reporting -> external combined-fair single-response CLI/reporting.

M52 seals the direct single-response fairness surface. The next highest-value specification frontier is **Milestone 53: textual multi-response temporal frontend**. The M11/M25/M28/M49 multi-response authorities already support no-fair, bounded/staged, and combined-fair verification, but external textual temporal syntax still represents only one response obligation at a time.

Acceptance criteria for M53:

- add a deterministic textual representation for conjunctions of named exact-action response clauses, with position-aware fail-closed diagnostics, escaping, and canonical rendering;
- compile directly to the sealed `MultiResponseProperty` semantics rather than adding another response traversal or acceptance interpretation;
- preserve exact no-fair, combined-fair, product-bounded, and staged model-before-product behavior, including clause identity, finite pending terminals, lasso evidence, and cutoff provenance;
- integrate the textual multi-response property with external declarative model files without changing the finite-model grammar;
- add independent generated differential evidence against direct multi-response verification plus parser/round-trip/malformed-input regressions;
- keep fairness as an external assumption surface, preserve historical single-response temporal grammar compatibility, and add no fairness-by-default behavior, wall-clock proof bound, or performance claim.

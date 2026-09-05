# Formal Verification Lab

A serious educational laboratory for building formal-methods machinery from first principles.

The project currently implements a small **explicit-state safety model checker** in Rust. It is intentionally not a wrapper around an existing model checker, and it is not presented as production-grade verification software.

## Implemented milestones

### Milestone 1: explicit-state transition systems

A model is a finite-state transition system with named state-variable metadata, one or more concrete initial states, an ordered labeled transition relation, and named safety invariants. Concrete state identity is defined by Rust `Eq + Hash`.

The canonical checker performs deterministic breadth-first search. It terminates on finite cyclic state spaces, checks invariants in deterministic order, and stores predecessor/action links so the first reachable invariant violation reconstructs a deterministic shortest transition-count counterexample.

### Milestone 2: bounded exploration with explicit incompleteness

`check_with_limits` supports optional `max_states`, `max_transitions`, and `max_depth` bounds without weakening the meaning of `SAFE`.

Results are explicit:

- `SAFE` — exhaustive reachable-state exploration completed and every invariant held;
- `VIOLATION` — a reachable state violated an invariant and a shortest counterexample is available;
- `INCONCLUSIVE` — a configured resource bound prevented exhaustive exploration.

Exact numeric bounds can still produce `SAFE` when they do not block required work. Resource exhaustion is never presented as a proof.

### Milestone 3: typed model construction and Peterson mutual exclusion

`TransitionSystemBuilder<S>` is a thin typed construction layer that always materializes the same canonical `TransitionSystem<S>` and therefore reuses canonical validation and checker semantics.

The first nontrivial consumer is a two-process Peterson mutual-exclusion model with explicit program counters, intent flags, and shared `turn`. The correct model exhaustively reaches 20 states and examines 34 transition edges while satisfying `mutual-exclusion`. A controlled lost-intent variant produces a reproducible six-transition counterexample ending with both processes in `Critical`.

### Milestone 4: exploration diagnostics and independent graph oracle

`CheckResult` exposes deterministic `max_depth_reached` and `transitions_by_action` diagnostics. The latter is a `BTreeMap`, so report ordering is stable, and its values sum to `explored_transitions`.

The checker is independently cross-checked across all 512 directed graphs on three labeled nodes. Those tests use Floyd–Warshall shortest paths—not a second BFS implementation—to validate reachability, maximum discovery depth, edge/action accounting, shortest violation distance, reconstructed trace validity, and repeated-run determinism.

### Milestone 5: differential sleep-set reduction audit

Milestone 5 introduces an **experimental reduction layer without promoting it to a trusted proof backend**.

`IndependenceRelation` is an explicit symmetric relation over complete action labels. No independence is inferred from prefixes or naming conventions. `audit_sleep_set_reduction` performs two executions:

1. the canonical exhaustive checker, which remains the authority;
2. an experimental deterministic depth-first sleep-set exploration using the supplied relation.

The audit succeeds only when both executions agree on verification status. A mismatch returns `ReductionAuditError::SemanticMismatch`; the reduced result is not allowed to overwrite or substitute for exhaustive proof evidence.

This fail-closed rule is deliberate because a user can declare a false independence relation. The regression suite contains such a model: exhaustive exploration reaches a violation, the intentionally unsound relation lets the reduced search miss it, and the audit must reject the experiment as a mismatch.

A separate `commuting-counters` product model supplies a genuinely commuting pair of actions (`left:increment`, `right:increment`). With that explicit relation, the current experiment preserves `SAFE` while reducing examined transition edges from 12 to 8 and recording 4 sleep-set prunes. These are graph-work counts, not a performance benchmark.

## Architecture

```text
src/model.rs      canonical transition-system abstraction and validation
src/builder.rs    thin typed construction layer
src/checker.rs    canonical deterministic BFS, bounds, diagnostics, traces
src/reduction.rs  opt-in experimental sleep-set exploration + exhaustive audit
src/examples.rs   executable teaching models and concurrent/product examples
src/report.rs     deterministic line-oriented checker reports
src/main.rs       CLI and option parsing; no transition-system semantics
tests/            semantic, builder, protocol, graph-oracle, reduction coverage
```

The canonical safety semantics remain in `model` + `checker`. The reduction module consumes those models but does not modify default `check()` / `check_with_limits()` behavior.

## Executable examples

List examples:

```bash
cargo run -- list
```

### Bounded counter

```bash
cargo run -- run counter
cargo run -- run counter --max-states 4 --max-transitions 3 --max-depth 3
```

The exact-bound run remains `SAFE`, reaches depth 3, and records three `increment` edges.

### Deliberately buggy simple mutual exclusion

```bash
cargo run -- run mutex-bug
cargo run -- run mutex-bug --max-depth 3
```

The unbounded command produces a shortest violation and exits 1. The depth-limited command exits 3 as `INCONCLUSIVE` rather than claiming safety.

### Peterson mutual exclusion

```bash
cargo run -- run peterson
cargo run -- run peterson-bug
```

The correct finite model is `SAFE` at 20 states / 34 examined edges / maximum BFS depth 6. The controlled lost-intent mutation reaches a mutual-exclusion violation.

### Commuting product and reduction audit

Canonical exhaustive run:

```bash
cargo run -- run commuting-counters
```

Differential reduction experiment:

```bash
cargo run -- reduce commuting-counters
```

The reduction command prints both exhaustive and reduced status/counts. It exits successfully only after the audit confirms matching status; this CLI surface is an experiment/audit, not a standalone proof command.

Expected stable markers include:

```text
reduction audit: MATCH
exhaustive states: 9
exhaustive transitions: 12
reduced states: 9
reduced transitions: 8
pruned transitions: 4
```

## CLI exit status

For canonical `run` commands:

- `0`: exhaustive result is `SAFE`;
- `1`: invariant `VIOLATION` with a counterexample;
- `2`: malformed CLI input or model/exploration/audit error;
- `3`: bounded canonical exploration is `INCONCLUSIVE`.

`reduce commuting-counters` currently uses the authoritative exhaustive result's success/violation exit status and treats differential mismatch as an error (exit 2).

## Tests

Run the same primary Rust checks used by CI:

```bash
cargo fmt --all -- --check
cargo build --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Coverage includes:

- reachable-state enumeration, cycles, invariant violations, shortest traces;
- multiple/duplicate initial states and model validation;
- honest state/transition/depth resource bounds;
- deterministic exploration diagnostics;
- typed-builder delegation to canonical model validation;
- correct and deliberately broken Peterson models;
- all 512 three-node directed graphs against an independent shortest-path oracle;
- explicit independence-relation validation and symmetry;
- an empty relation degenerating to unpruned exploration;
- a commuting product where the experimental sleep set reduces examined edges;
- an intentionally false independence declaration that must fail closed with `SemanticMismatch`.

CI additionally exercises the canonical violation path, bounded-inconclusive path, Peterson proof/counterexample paths, diagnostic markers, and the real `reduce commuting-counters` CLI audit.

## Model and reduction trust boundaries

Canonical model construction rejects malformed metadata and empty transition labels. The checker propagates transition-generation errors. It still cannot prove that a user transition function is pure, finite, deterministic, or a faithful representation of an external implementation.

Reduction declarations have an even narrower trust boundary: an `IndependenceRelation` is a **claim supplied by the model author**, not something established from action strings. Milestone 5 therefore keeps exhaustive checking in the loop and rejects any observed status mismatch. The current reduced engine must not be used by itself to assert safety.

## Limitations

- explicit-state memory grows with retained reachable states;
- safety invariants only; Peterson liveness/starvation freedom is not claimed;
- protocol results apply to the finite model and its atomic-step assumptions, not arbitrary machine code or weak-memory executions;
- resource limits do not interrupt a successor function while it is building one state's transition vector;
- reduction statistics are state/edge accounting, not controlled performance benchmarks;
- the sleep-set engine is experimental and differentially audited, not a standalone trusted POR backend;
- no LTL/CTL, fairness, Büchi automata, SAT/SMT, BDDs, symbolic execution, theorem proving, symmetry reduction, disk-backed storage, parallel search, or distributed checking;
- deterministic traces require deterministic successor ordering from the model.

## Roadmap

Milestones 1–5 now cover canonical explicit-state safety checking, honest bounded outcomes, typed construction, a real concurrent protocol, independent BFS validation, and a fail-closed partial-order reduction experiment.

The next architectural promotion should deepen **property expressiveness** rather than farming more reduction examples. The highest-value candidate is a small temporal-property vertical slice:

1. define a deliberately narrow temporal property such as reachability/eventual-state existence or invariant-plus-deadlock detection with precise finite-state semantics;
2. keep safety checking backward-compatible and avoid claiming full LTL/CTL before an actual parser/automaton implementation exists;
3. add executable counterexample/witness semantics and deterministic reporting;
4. validate the property engine against independent graph oracles/generated finite models;
5. only then consider Büchi/LTL machinery or a trusted standalone POR path with stronger independence evidence.

# Formal Verification Lab

A serious educational laboratory for building formal-methods machinery from first principles.

The project implements a small **explicit-state formal-verification laboratory** in Rust. It is not a wrapper around an existing model checker and is not presented as production-grade verification software.

## Implemented milestones

### Milestone 1: explicit-state transition systems

A model is a finite-state transition system with named state-variable metadata, one or more concrete initial states, an ordered labeled transition relation, and named safety invariants. Concrete state identity is defined by Rust `Eq + Hash`.

The canonical checker performs deterministic breadth-first search (BFS). It terminates on finite cyclic state spaces, checks invariants in deterministic order, and stores predecessor/action links so the first reachable invariant violation reconstructs a deterministic shortest transition-count counterexample.

### Milestone 2: bounded exploration with explicit incompleteness

`check_with_limits` supports optional `max_states`, `max_transitions`, and `max_depth` bounds without weakening the meaning of `SAFE`.

Results are explicit:

- `SAFE` — exhaustive reachable-state exploration completed and every invariant held;
- `VIOLATION` — a reachable state violated an invariant and a shortest counterexample is available;
- `INCONCLUSIVE` — a configured resource bound prevented exhaustive exploration.

Exact numeric bounds can still produce `SAFE` when they do not block required work. Resource exhaustion is never presented as a proof.

### Milestone 3: typed model construction and Peterson mutual exclusion

`TransitionSystemBuilder<S>` is a thin typed construction layer that materializes the same canonical `TransitionSystem<S>` and therefore reuses canonical validation and checker semantics.

The first nontrivial consumer is a two-process Peterson mutual-exclusion model with explicit program counters, intent flags, and shared `turn`. The correct model exhaustively reaches 20 states and examines 34 transition edges while satisfying `mutual-exclusion`. A controlled lost-intent variant produces a reproducible six-transition counterexample ending with both processes in `Critical`.

### Milestone 4: exploration diagnostics and independent graph oracle

`CheckResult` exposes deterministic `max_depth_reached` and `transitions_by_action` diagnostics. The latter is a `BTreeMap`, so report ordering is stable, and its values sum to `explored_transitions`.

The checker is independently cross-checked across all 512 directed graphs on three labeled nodes. Those tests use Floyd–Warshall shortest paths—not a second BFS implementation—to validate reachability, maximum discovery depth, edge/action accounting, shortest violation distance, reconstructed trace validity, and repeated-run determinism.

### Milestone 5: differential sleep-set reduction audit

Milestone 5 adds an **experimental reduction layer without promoting it to a trusted proof backend**.

`IndependenceRelation` is an explicit symmetric relation over complete action labels. `audit_sleep_set_reduction` always runs both the canonical exhaustive checker and an experimental deterministic sleep-set DFS. The audit succeeds only when their verification statuses agree. A mismatch returns `ReductionAuditError::SemanticMismatch`; reduced search cannot overwrite exhaustive proof evidence.

The `commuting-counters` product model provides a genuinely commuting pair (`left:increment`, `right:increment`). With that relation the experiment preserves `SAFE` while reducing examined transition edges from 12 to 8 and recording 4 sleep-set prunes. A deliberately false independence declaration has its own regression and must fail closed. These are graph-work counts, not performance benchmarks.

### Milestone 6: existential reachability properties

Milestone 6 expands property expressiveness with a deliberately narrow query:

> **Does at least one reachable state satisfy a target predicate?**

`ReachabilityProperty<S>` names a target predicate and `check_reachability` returns either:

- `REACHABLE` with a deterministic shortest transition-count witness; or
- `UNREACHABLE` only after the finite reachable graph has been exhaustively explored.

The property engine does **not** implement a second graph traversal. Internally it creates a derived view over the same transition relation with one sentinel invariant that holds until the target is seen, then delegates to the canonical BFS checker. A target hit is therefore the canonical checker's first sentinel-invariant violation and inherits its shortest-path and deterministic-order guarantees.

The derived-model hook is `pub(crate)` rather than public API. It reuses the same model metadata validation and the same transition-relation `Arc`; reachability is an additional interpretation over an existing transition graph, not an alternate execution semantics.

Reachability queries intentionally replace the model's safety invariants while evaluating the target. This means a query answers graph reachability independently of whether the original model is safe. Safety and reachability remain separate properties.

The reachability suite again exhaustively enumerates all 512 directed three-node graphs. An independent Floyd–Warshall oracle validates target reachability, shortest witness length, witness-edge validity, unreachable-state exhaustion counts, and repeated-run determinism.

This milestone is **not** full LTL/CTL and does not claim universal eventuality, fairness, progress, or liveness. `UNREACHABLE` means no target state exists in the exhaustively explored finite reachable graph.

## Architecture

```text
src/model.rs      canonical transition-system abstraction and validation
src/builder.rs    thin typed construction layer
src/checker.rs    canonical deterministic BFS, bounds, diagnostics, traces
src/property.rs   reachability queries encoded through the canonical checker
src/reduction.rs  opt-in experimental sleep-set exploration + exhaustive audit
src/examples.rs   executable teaching models and concurrent/product examples
src/report.rs     deterministic checker and property reports
src/main.rs       CLI and option parsing; no graph traversal semantics
tests/            semantic, builder, protocol, graph-oracle, reduction,
                  and reachability coverage
```

The canonical graph semantics remain in `model` + `checker`. Property and reduction layers consume those models without changing default `check()` / `check_with_limits()` behavior.

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

```bash
cargo run -- run commuting-counters
cargo run -- reduce commuting-counters
```

The reduction audit reports matching exhaustive/reduced status and the stable exploration counts:

```text
exhaustive states: 9
exhaustive transitions: 12
reduced states: 9
reduced transitions: 8
pruned transitions: 4
```

### Reachability witnesses and exhaustive absence

A reachable target:

```bash
cargo run -- reach counter-three
```

The command reports `reachability: REACHABLE`, exits 0, and prints the shortest three-transition witness from counter value 0 to value 3.

An absent target:

```bash
cargo run -- reach counter-four
```

The finite counter can only reach 0 through 3. The command therefore exhausts all four reachable states, reports `reachability: UNREACHABLE`, prints `witness: none (reachable graph exhausted)`, and exits 4.

## CLI exit status

Canonical `run` commands:

- `0`: exhaustive result is `SAFE`;
- `1`: invariant `VIOLATION` with a counterexample;
- `2`: malformed CLI input or model/exploration/audit/property error;
- `3`: bounded canonical exploration is `INCONCLUSIVE`.

Reachability commands:

- `0`: target is `REACHABLE` and a shortest witness is available;
- `4`: target is `UNREACHABLE` after exhaustive finite-state exploration.

`reduce commuting-counters` uses the authoritative exhaustive result's success/violation exit status and treats differential mismatch as an error (exit 2).

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
- all 512 three-node directed graphs against an independent shortest-path oracle for canonical BFS;
- explicit reduction independence validation, positive edge reduction, and fail-closed semantic mismatch;
- reachability property-name validation;
- zero-transition initial-state reachability witnesses;
- deterministic shortest reachability witnesses;
- unreachable targets requiring full reachable-graph exhaustion;
- reachability queries remaining independent of original safety invariants;
- all 512 three-node directed graphs independently cross-checking reachability and witness semantics.

CI additionally exercises canonical violation, bounded-inconclusive, Peterson, reduction-audit, reachable-property, and unreachable-property paths through the real CLI.

## Trust boundaries

Canonical model construction rejects malformed metadata and empty transition labels. The checker propagates transition-generation errors. It cannot prove that a user transition function is pure, finite, deterministic, or faithful to an external implementation.

Reduction declarations are model-author claims; M5 therefore retains exhaustive checking as authority and rejects observed status mismatch.

Reachability predicates are also user-supplied Rust functions. M6 proves only whether such a predicate is encountered in the finite explicit transition graph represented by the model. It does not infer real-world liveness or implementation behavior from that result.

## Limitations

- explicit-state memory grows with retained reachable states;
- safety plus existential state reachability only; no universal eventuality or general temporal logic;
- Peterson liveness/starvation freedom is not claimed;
- protocol results apply to the finite model and its atomic-step assumptions, not arbitrary machine code or weak-memory executions;
- resource limits do not interrupt a successor function while it is building one state's transition vector;
- reduction statistics are state/edge accounting, not controlled performance benchmarks;
- the sleep-set engine is experimental and differentially audited, not a standalone trusted POR backend;
- no LTL/CTL parser, fairness, Büchi automata, SAT/SMT, BDDs, symbolic execution, theorem proving, symmetry reduction, disk-backed storage, parallel search, or distributed checking;
- deterministic traces require deterministic successor ordering from the model.

## Roadmap

Milestones 1–6 now cover deterministic explicit-state safety, honest bounded outcomes, typed construction, a real concurrent protocol, independent BFS validation, fail-closed reduction experiments, and existential reachability with shortest witnesses.

The next highest-value property frontier is **deadlock/terminal-state analysis** rather than immediately claiming full temporal logic:

1. define deadlock precisely as a reachable state with no enabled outgoing transition, keeping legitimate terminal states distinguishable through an explicit predicate or policy;
2. return a deterministic shortest witness to a reachable deadlock when one exists;
3. prove deadlock absence only after exhaustive finite-state exploration;
4. cross-check deadlock detection against independently generated finite-graph out-degree oracles;
5. integrate the property into CLI/reporting before considering broader temporal operators, fairness, Büchi/LTL machinery, or stronger standalone POR claims.

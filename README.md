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

The property engine does **not** implement a second graph traversal. Internally it creates a derived view over the same transition relation with one sentinel invariant that holds until the target is seen, then delegates to the canonical BFS checker. A target hit therefore inherits canonical shortest-path and deterministic-order guarantees.

Reachability queries intentionally replace the model's safety invariants while evaluating the target. This means a query answers graph reachability independently of whether the original model is safe. Safety and reachability remain separate properties.

The reachability suite exhaustively enumerates all 512 directed three-node graphs. An independent Floyd–Warshall oracle validates target reachability, shortest witness length, witness-edge validity, unreachable-state exhaustion counts, and repeated-run determinism.

This milestone is **not** full LTL/CTL and does not claim universal eventuality, fairness, progress, or liveness. `UNREACHABLE` means no target state exists in the exhaustively explored finite reachable graph.

### Milestone 7: deadlock and legitimate terminal-state analysis

Milestone 7 adds a second finite-state property without equating every terminal state with a bug.

`DeadlockProperty<S>` contains an explicit **allowed-terminal predicate**. A reachable state is classified as a deadlock exactly when:

1. its transition relation produces no outgoing transitions; and
2. the allowed-terminal predicate returns `false` for that state.

`check_deadlock` returns either:

- `DEADLOCK_FOUND` with a deterministic shortest transition-count witness; or
- `DEADLOCK_FREE` only after exhaustive unbounded exploration of the finite reachable graph.

The implementation does not call a model's transition function twice to decide whether a state is terminal. Milestone 7 factors the existing BFS loop into one crate-private canonical `search_with_probes` substrate. Safety checking continues to use that substrate before successor generation, while deadlock analysis observes the single generated successor vector before its edges are expanded. There is still one canonical BFS implementation and one transition-generation call per checked state.

Deadlock analysis intentionally ignores the model's original safety invariants, just as reachability is a distinct graph property. A safety violation therefore cannot hide a later reachable deadlock from a deadlock query.

The deadlock suite independently exhausts all 512 directed three-node graphs. Floyd–Warshall supplies shortest reachable distances while a separately computed out-degree oracle identifies terminal nodes. Tests verify deadlock existence, shortest witness length, witness-edge validity, deterministic repeated results, and full state/edge accounting when no reachable deadlock exists.

This milestone is **not** starvation freedom, livelock detection, fairness, or general liveness. `DEADLOCK_FREE` means only that every reachable out-degree-zero state accepted by the finite model is explicitly allowed by the supplied terminal policy.

## Architecture

```text
src/model.rs      canonical transition-system abstraction and validation
src/builder.rs    thin typed construction layer
src/checker.rs    one canonical deterministic BFS substrate, bounds,
                  diagnostics, invariant checking, predecessor traces
src/property.rs   existential reachability + deadlock/terminal policies
src/reduction.rs  opt-in experimental sleep-set exploration + exhaustive audit
src/examples.rs   executable teaching models and concurrent/product examples
src/report.rs     deterministic checker, reachability, and deadlock reports
src/main.rs       CLI and option parsing; no graph traversal semantics
tests/            semantic, builder, protocol, graph-oracle, reduction,
                  reachability, and deadlock coverage
```

The canonical graph semantics remain in `model` + `checker`. Property layers consume the same transition graph and canonical BFS substrate. Reduction remains an explicitly experimental, differentially audited path.

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

### Deadlock versus legitimate termination

Treat counter value 3 as an intentional terminal state:

```bash
cargo run -- deadlock counter-terminal-ok
```

The command exhausts all four states and reports `deadlock: DEADLOCK_FREE`.

Apply a strict policy that allows no terminal state:

```bash
cargo run -- deadlock counter-terminal-forbidden
```

The same graph now reports `deadlock: DEADLOCK_FOUND`, exits 5, and prints the shortest three-transition witness to counter value 3. The difference is the explicit terminal policy, not a change to transition semantics.

## CLI exit status

Canonical `run` commands:

- `0`: exhaustive result is `SAFE`;
- `1`: invariant `VIOLATION` with a counterexample;
- `2`: malformed CLI input or model/exploration/audit/property error;
- `3`: bounded canonical exploration is `INCONCLUSIVE`.

Reachability commands:

- `0`: target is `REACHABLE` and a shortest witness is available;
- `4`: target is `UNREACHABLE` after exhaustive finite-state exploration.

Deadlock commands:

- `0`: no unexpected terminal state exists after exhaustive exploration;
- `5`: a reachable unexpected terminal state was found with a shortest witness.

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
- reachability property-name validation, shortest witnesses, and exhaustive absence;
- reachability remaining independent of original safety invariants;
- all 512 three-node directed graphs independently cross-checking reachability semantics;
- deadlock property-name validation and zero-transition initial deadlocks;
- legitimate-terminal versus strict-terminal policies on the same executable counter model;
- deadlock queries remaining independent of original safety invariants;
- all 512 three-node directed graphs cross-checked against independent shortest-distance and out-degree deadlock oracles.

CI additionally exercises canonical violation, bounded-inconclusive, Peterson, reduction-audit, reachable/unreachable property paths, and deadlock-free/deadlock-found terminal policies through the real CLI.

## Trust boundaries

Canonical model construction rejects malformed metadata and empty transition labels. The checker propagates transition-generation errors. It cannot prove that a user transition function is pure, finite, deterministic, or faithful to an external implementation.

Reduction declarations are model-author claims; M5 therefore retains exhaustive checking as authority and rejects observed status mismatch.

Reachability predicates are user-supplied Rust functions. M6 proves only whether such a predicate is encountered in the finite explicit transition graph represented by the model.

Deadlock terminal policies are also model-author claims. Marking a terminal state as allowed is an explicit semantic assertion by the caller; the checker does not infer whether that terminal state represents successful completion in an external implementation.

## Limitations

- explicit-state memory grows with retained reachable states;
- safety, existential state reachability, and finite-state deadlock/terminal analysis only; no general temporal logic;
- deadlock freedom does not imply starvation freedom, livelock freedom, fairness, or progress;
- Peterson liveness/starvation freedom is not claimed;
- protocol results apply to the finite model and its atomic-step assumptions, not arbitrary machine code or weak-memory executions;
- resource limits do not interrupt a successor function while it is building one state's transition vector;
- reduction statistics are state/edge accounting, not controlled performance benchmarks;
- the sleep-set engine is experimental and differentially audited, not a standalone trusted POR backend;
- no LTL/CTL parser, fairness, Büchi automata, SAT/SMT, BDDs, symbolic execution, theorem proving, symmetry reduction, disk-backed storage, parallel search, or distributed checking;
- deterministic traces require deterministic successor ordering from the model.

## Roadmap

Milestones 1–7 cover deterministic explicit-state safety, honest bounded outcomes, typed construction, a real concurrent protocol, independent BFS validation, fail-closed reduction experiments, existential reachability, and explicit deadlock/terminal-state analysis.

The next architectural frontier should add **cycle and recurrent-state structure** rather than proliferating more terminal-policy examples. A high-value Milestone 8 would:

1. compute reachable strongly connected components (SCCs) with deterministic reporting;
2. distinguish trivial terminal states from cyclic recurrent regions and identify reachable nontrivial cycles/self-loops;
3. return a deterministic stem-plus-cycle witness for a reachable recurrent region;
4. cross-check SCC/cycle classification against an independent generated-graph oracle;
5. use that executable foundation before attempting universal eventuality, livelock properties, Büchi/LTL machinery, or fairness claims.

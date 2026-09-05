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

`DeadlockProperty<S>` contains an explicit **allowed-terminal predicate**. A reachable state is classified as a deadlock exactly when its transition relation produces no outgoing transitions and the allowed-terminal predicate returns `false` for that state.

`check_deadlock` returns either:

- `DEADLOCK_FOUND` with a deterministic shortest transition-count witness; or
- `DEADLOCK_FREE` only after exhaustive unbounded exploration of the finite reachable graph.

The implementation does not call a model's transition function twice to decide whether a state is terminal. Milestone 7 factors the existing BFS loop into one crate-private canonical `search_with_probes` substrate. Safety checking continues to use that substrate before successor generation, while deadlock analysis observes the single generated successor vector before its edges are expanded. There is still one canonical BFS implementation and one transition-generation call per checked state.

The deadlock suite independently exhausts all 512 directed three-node graphs. Floyd–Warshall supplies shortest reachable distances while a separately computed out-degree oracle identifies terminal nodes.

This milestone is **not** starvation freedom, livelock detection, fairness, or general liveness.

### Milestone 8: reachable SCCs and recurrent-cycle witnesses

Milestone 8 adds structural analysis of the exhaustively reachable graph.

`analyze_recurrence` uses the canonical BFS substrate once to capture states in deterministic discovery order and each checked state's already-generated successor vector. After that single model exploration, all SCC and witness work runs only over the in-memory snapshot; the model transition function is not invoked again.

The recurrence subsystem computes every reachable strongly connected component with Tarjan's algorithm. For deterministic reporting, states inside a component and components themselves are ordered by canonical BFS discovery index. A component is marked `cyclic` exactly when it contains more than one state or a singleton state has a self-loop.

When at least one cyclic SCC exists, the first cyclic component in that canonical ordering receives a `CycleWitness`:

- `stem` is a shortest transition-count path from the declared initial-state set to the component's lowest-discovery entry state;
- `cycle` starts and ends at that same entry and follows real labeled snapshot edges;
- cycle selection is deterministic but is not claimed to be the globally shortest possible cycle.

The SCC suite again exhausts all 512 directed three-node graphs, but does not use a second SCC algorithm as the oracle. Floyd–Warshall determines mutual reachability: two reachable nodes belong to the same expected SCC exactly when each can reach the other. A separate self-loop test classifies singleton SCCs as cyclic or acyclic. The suite validates the complete SCC partition, cycle classification, snapshot state/edge accounting, deterministic repeated results, shortest stem distance, and closed witness edges.

A recurrent cycle is only graph structure. Its existence does **not** by itself establish livelock, starvation, unfairness, or failure of an eventuality property.

## Architecture

```text
src/model.rs       canonical transition-system abstraction and validation
src/builder.rs     thin typed construction layer
src/checker.rs     one canonical deterministic BFS substrate, bounds,
                   diagnostics, invariant checking, predecessor traces
src/property.rs    existential reachability + deadlock/terminal policies
src/recurrence.rs  single-exploration graph snapshot + deterministic SCCs,
                   shortest stems, closed cycle witnesses
src/reduction.rs   opt-in experimental sleep-set exploration + exhaustive audit
src/examples.rs    executable teaching models and concurrent/product examples
src/report.rs      deterministic checker/property/SCC reports
src/main.rs        CLI and option parsing; no model traversal semantics
tests/             semantic, protocol, graph-oracle, reduction, property,
                   deadlock, and SCC recurrence coverage
```

The canonical model traversal remains in `model` + `checker`. Higher-level analyses either delegate directly to it or capture its generated graph once and operate on the resulting finite snapshot.

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

The reduction audit reports matching exhaustive/reduced status and stable graph-work counts: exhaustive 9 states / 12 transitions, reduced 9 states / 8 transitions, with 4 pruned transitions.

### Reachability witnesses and exhaustive absence

```bash
cargo run -- reach counter-three
cargo run -- reach counter-four
```

The first command reports `REACHABLE` with the shortest three-transition witness. The second exhausts the four-state finite counter and reports `UNREACHABLE` with exit 4.

### Deadlock versus legitimate termination

```bash
cargo run -- deadlock counter-terminal-ok
cargo run -- deadlock counter-terminal-forbidden
```

The first policy explicitly permits value 3 as successful termination and is `DEADLOCK_FREE`; the strict second policy reports `DEADLOCK_FOUND` with exit 5 and a three-transition witness.

### SCC and recurrent-cycle structure

Acyclic finite counter:

```bash
cargo run -- scc counter
```

Expected structural markers include `recurrence: ACYCLIC`, `scc count: 4`, `cyclic scc count: 0`, and `cycle witness: none`.

Cyclic traffic-light model:

```bash
cargo run -- scc traffic-light
```

The three reachable light states form one cyclic SCC. The report includes `recurrence: CYCLIC`, `scc count: 1`, `cyclic scc count: 1`, a zero-transition stem at `Red`, and a closed three-`advance` cycle back to `Red`. Both SCC commands exit 0 because cycle presence is structural information, not an error status.

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

SCC commands use exit 0 for either cyclic or acyclic successful analysis. `reduce commuting-counters` uses the authoritative exhaustive result's status and treats differential mismatch as an error (exit 2).

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
- multiple/duplicate initial states and honest state/transition/depth bounds;
- deterministic exploration diagnostics and builder delegation;
- correct and deliberately broken Peterson models;
- all 512 three-node graphs against independent shortest-path/accounting oracles;
- explicit reduction validation, positive edge reduction, and fail-closed mismatch;
- reachability shortest witnesses, exhaustive absence, and 512-graph oracle coverage;
- legitimate-terminal versus strict deadlock policies and 512-graph out-degree oracle coverage;
- bounded-counter acyclic SCC decomposition and traffic-light recurrent-cycle witness;
- all 512 three-node graphs cross-checked against independent Floyd–Warshall mutual-reachability SCC partitions and self-loop cycle classification.

CI additionally exercises canonical violation, bounded-inconclusive, Peterson, reduction-audit, reachability, deadlock, and both acyclic/cyclic SCC paths through the real CLI.

## Trust boundaries

Canonical model construction rejects malformed metadata and empty transition labels. The checker propagates transition-generation errors. It cannot prove that a user transition function is pure, finite, deterministic, or faithful to an external implementation.

Reduction declarations are model-author claims; M5 therefore retains exhaustive checking as authority and rejects observed status mismatch.

Reachability predicates and deadlock terminal policies are user-supplied semantic assertions over the modeled finite graph.

SCC analysis is structural: it faithfully classifies the captured reachable graph, subject to the same transition-model trust boundary. A cyclic SCC says executions can remain within a recurrent region; it does not say an external implementation will take those transitions, that a scheduler is fair or unfair, or that useful progress is absent.

## Limitations

- explicit-state memory grows with retained reachable states; SCC analysis additionally retains the complete reachable labeled edge snapshot;
- Tarjan SCC discovery currently uses recursive DFS over the captured snapshot, so extremely deep graphs may require a future iterative implementation;
- safety, existential reachability, finite-state deadlock policies, and structural SCC/cycle analysis only; no general temporal logic yet;
- deadlock freedom and cycle presence do not imply starvation/livelock/fairness/progress results;
- Peterson liveness/starvation freedom is not claimed;
- protocol results apply to the finite model and its atomic-step assumptions, not arbitrary machine code or weak-memory executions;
- resource limits do not interrupt a successor function while it is building one state's transition vector;
- reduction statistics are state/edge accounting, not controlled performance benchmarks;
- the sleep-set engine is experimental and differentially audited, not a standalone trusted POR backend;
- no LTL/CTL parser, fairness, Büchi automata, SAT/SMT, BDDs, symbolic execution, theorem proving, symmetry reduction, disk-backed storage, parallel search, or distributed checking;
- deterministic traces require deterministic successor ordering from the model.

## Roadmap

Milestones 1–8 now cover deterministic explicit-state safety, honest bounded outcomes, typed construction, a real concurrent protocol, independent BFS validation, fail-closed reduction experiments, existential reachability, explicit deadlock policy analysis, and deterministic SCC/recurrent-cycle structure.

The next high-value architectural promotion is a deliberately narrow **universal eventuality** property over the finite graph rather than a premature full LTL surface. A Milestone 9 should:

1. define precise finite-path semantics for "from every initial execution, a target state is eventually reached";
2. classify a reachable non-target terminal state as a finite counterexample and a reachable non-target cyclic SCC as an infinite stem-plus-cycle counterexample;
3. reuse the one-exploration graph snapshot/SCC machinery rather than invoking the transition function again;
4. cross-check positive and negative results against independently generated finite graphs;
5. state fairness assumptions explicitly and avoid claiming full LTL/CTL until parser/automaton machinery actually exists.

# Formal Verification Lab

A serious educational laboratory for building formal-methods machinery from first principles.

The project currently implements a small **explicit-state safety model checker** in Rust. It is intentionally not a wrapper around an existing model checker, and it is not presented as production-grade verification software.

## Implemented milestones

### Milestone 1: explicit-state transition systems

A model is a finite-state transition system with four explicit pieces:

1. **State variables** — named metadata describing the logical components of a state.
2. **Initial states** — one or more concrete starting states.
3. **Transition relation** — a function mapping a state to an ordered list of labeled successor states.
4. **Safety invariants** — named predicates that must hold in every reachable state.

The concrete Rust state type must implement `Clone + Eq + Hash + Debug`. Equality and hashing define state identity for visited-state detection.

The checker performs deterministic breadth-first search (BFS): initial states, invariants, and successors are processed in declaration order; already-visited states are not enqueued again; and finite cyclic state spaces therefore terminate. Each newly discovered state retains its predecessor and incoming action, so the first reachable invariant violation reconstructs a deterministic shortest transition-count counterexample.

### Milestone 2: bounded exploration with explicit incompleteness

`check_with_limits` adds optional `max_states`, `max_transitions`, and `max_depth` bounds without weakening the meaning of `SAFE`.

Results are explicit:

- `SAFE` — exhaustive reachable-state exploration completed and every invariant held;
- `VIOLATION` — a reachable state violated an invariant and a shortest counterexample is available;
- `INCONCLUSIVE` — a configured resource bound prevented exhaustive exploration.

A limit does not make a result inconclusive merely because its numeric value is reached. `INCONCLUSIVE` is returned only when the limit blocks work needed to finish the proof. Exact-bound closed state spaces can still be `SAFE`; resource exhaustion is never reported as a safety proof.

### Milestone 3: typed model construction and Peterson mutual exclusion

`TransitionSystemBuilder<S>` is a thin generic construction layer for Rust models. It collects state-variable metadata, initial states, and safety invariants around one typed transition relation, then materializes the same canonical `TransitionSystem<S>`. `build()` delegates to `TransitionSystem::new`, so builder-based models do not bypass existing validation or create a second execution semantics.

The first nontrivial consumer is a two-process model of **Peterson's mutual-exclusion algorithm**. The state explicitly contains:

- a program counter for each process;
- one intent flag per process;
- the shared `turn` variable.

Each assignment or control-flow move is modeled as an atomic transition. A blocked Peterson wait has no enabled `enter` edge; explicit stuttering is omitted because it does not change safety reachability.

The correct model exhaustively reaches 20 states and 34 transition edges and satisfies `mutual-exclusion`. A controlled `peterson-bug` variant changes only the request step: it clears instead of sets the process intent flag. The same checker then finds a reproducible shortest trace in which both processes reach `Critical`.

This paired model is important: the project now demonstrates both proof of a known finite-state concurrent protocol under the modeled assumptions and automatic diagnosis when one protocol assumption is deliberately broken.

## Architecture

```text
src/model.rs      canonical transition-system abstraction and validation
src/builder.rs    thin typed construction layer that builds model.rs systems
src/checker.rs    deterministic BFS, resource bounds, visited-state tracking,
                  invariant checks, predecessor storage, shortest traces
src/examples.rs   executable teaching models, including Peterson
src/report.rs     deterministic text rendering (presentation layer)
src/main.rs       CLI and option parsing only; no transition-system semantics
tests/            semantic, builder, protocol, and integration coverage
```

The semantic core (`model` + `checker`) does not depend on the CLI or output formatting. The builder also does not contain checker logic.

## Executable examples

List examples:

```bash
cargo run -- list
```

### Correct bounded counter

```bash
cargo run -- run counter
cargo run -- run counter --max-states 4 --max-transitions 3 --max-depth 3
```

The counter reaches values `0, 1, 2, 3` and satisfies `value <= 3`. The second command demonstrates an exact-bound proof that remains `SAFE`.

### Deliberately buggy simple mutual exclusion

```bash
cargo run -- run mutex-bug
cargo run -- run mutex-bug --max-depth 3
```

The unbounded command prints a shortest mutual-exclusion counterexample and exits 1. Stopping before the violation depth prints `INCONCLUSIVE` and exits 3 rather than claiming safety.

### Cyclic traffic light

```bash
cargo run -- run traffic-light
```

The model cycles `Red -> Green -> Yellow -> Red`; visited-state detection proves the reachable graph contains only three states.

### Peterson mutual exclusion

```bash
cargo run -- run peterson
```

The correct model is exhaustively checked and reports `SAFE`.

The controlled lost-intent mutation is executable separately:

```bash
cargo run -- run peterson-bug
```

Its shortest counterexample has this action shape:

```text
p0:set-flag
p0:set-turn
p0:enter
p1:set-flag
p1:set-turn
p1:enter
```

The final state has both program counters at `Critical`, so the CLI reports `VIOLATION` and exits 1.

## CLI exit status

- `0`: exhaustive result is `SAFE`;
- `1`: invariant `VIOLATION` with a counterexample;
- `2`: malformed CLI input or model/exploration error;
- `3`: bounded exploration is `INCONCLUSIVE`.

Available exploration options are:

```text
--max-states N
--max-transitions N
--max-depth N
```

Options may be combined. Duplicate or malformed options are rejected rather than silently overridden.

## Tests

Run the same checks used by CI:

```bash
cargo fmt --all -- --check
cargo build --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Coverage includes:

- reachable-state enumeration and cyclic termination;
- invariant success/failure and shortest counterexample reconstruction;
- multiple and duplicate initial states;
- deterministic checker/report output;
- malformed model metadata and transition labels;
- state, transition, and depth exhaustion returning `INCONCLUSIVE`;
- exact-bound exhaustive proofs still returning `SAFE`;
- builder construction producing an executable canonical transition system;
- builder input receiving the same canonical model validation;
- exhaustive Peterson mutual-exclusion safety;
- exact-bound Peterson exploration;
- the controlled Peterson mutation producing a deterministic shortest mutex counterexample.

CI additionally executes the simple counterexample path, bounded-inconclusive path, correct Peterson proof, and buggy Peterson counterexample through the real CLI and verifies their exit codes and stable report markers.

## Model validation

Construction rejects empty model names, missing state variables, empty/duplicate state-variable names, missing initial states, missing invariants, and empty/duplicate invariant names. Exploration rejects transitions with empty action labels and propagates model-supplied transition-generation errors.

`TransitionSystemBuilder` intentionally does not duplicate these rules; it delegates to canonical model construction at `build()` time.

Validation remains modest: the checker does not prove that a user-supplied transition function is pure, finite, total, deterministic, or a faithful representation of an external implementation.

## Limitations

The current checker remains deliberately focused:

- explicit-state only; memory usage grows with retained reachable states;
- safety invariants only; Peterson liveness/starvation freedom is **not** claimed;
- protocol results apply to the finite model and its atomic-step assumptions, not arbitrary machine code or weak-memory executions;
- resource limits do not interrupt a user transition function while it is constructing one state's successor vector;
- no LTL/CTL, fairness, Büchi automata, or temporal-property compiler;
- no SAT/SMT, BDDs, symbolic execution, theorem proving, or partial-order reduction;
- no large external DSL or parser; models are typed Rust values/functions;
- no symmetry reduction, disk-backed state storage, parallel search, or distributed checking;
- deterministic traces require deterministic successor ordering from the model.

These omissions are intentional and belong to later milestones.

## Roadmap

Milestones 1–3 establish deterministic explicit-state safety checking, honest bounded-search outcomes, a reusable typed construction layer, and a real concurrent-protocol case study.

The next highest-value frontier is **exploration observability plus algorithmic self-checking**, without changing proof semantics:

1. expose frontier depth and deterministic per-action transition counts in `CheckResult`;
2. add graph-level/property-style tests that independently validate BFS shortest-path and accounting invariants across generated finite transition systems;
3. use those diagnostics to compare correct and mutated concurrent models without pretending CI timing is a benchmark;
4. then evaluate a small, well-scoped reduction technique such as sleep-set/partial-order reduction only if equivalence can be checked against exhaustive exploration;
5. temporal logic and symbolic back ends remain later architectural phases.

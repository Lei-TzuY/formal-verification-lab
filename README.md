# Formal Verification Lab

A serious educational laboratory for building formal-methods machinery from first principles.

Milestone 1 implements a small **explicit-state safety model checker** in Rust. It is intentionally not a wrapper around an existing model checker, and it is not presented as production-grade verification software.

## Milestone 1: explicit-state transition systems

A model is a finite-state transition system with four explicit pieces:

1. **State variables** — named metadata describing the logical components of a state.
2. **Initial states** — one or more concrete starting states.
3. **Transition relation** — a function mapping a state to an ordered list of labeled successor states.
4. **Safety invariants** — named predicates that must hold in every reachable state.

The concrete Rust state type must implement `Clone + Eq + Hash + Debug`. Equality and hashing define state identity for visited-state detection.

### Exploration semantics

The checker performs breadth-first search (BFS):

- initial states are enqueued in declaration order;
- duplicate initial states are ignored after their first occurrence;
- invariants are checked in declaration order when each state is dequeued;
- successor transitions are considered in the order returned by the model;
- previously visited states are not enqueued again;
- finite reachable state spaces therefore terminate, including cyclic systems.

For each newly discovered state, the checker stores a predecessor node and the action used to reach it. If an invariant fails, those predecessor links reconstruct a counterexample from an initial state. Because exploration is BFS, the first reported violation has a **shortest number-of-transitions trace**. Ties among equally short traces are deterministic given deterministic model successor ordering.

## Architecture

```text
src/model.rs      transition-system abstraction and model validation
src/checker.rs    deterministic BFS, visited-state tracking, invariant checks,
                  predecessor storage, shortest counterexample reconstruction
src/examples.rs   executable teaching models
src/report.rs     deterministic text rendering (presentation layer)
src/main.rs       small CLI only; no semantic logic
tests/            integration coverage for checker semantics and validation
```

The semantic core (`model` + `checker`) does not depend on the CLI or output formatting.

## Executable examples

List examples:

```bash
cargo run -- list
```

### Correct bounded counter

```bash
cargo run -- run counter
```

The counter reaches values `0, 1, 2, 3` and satisfies `value <= 3`.

### Deliberately buggy mutual exclusion

```bash
cargo run -- run mutex-bug
```

Each process independently moves `Idle -> Trying -> Critical -> Idle`. The bug is that entering `Critical` does not check the other process, so the checker reaches a state where both processes are critical. The CLI prints a reproducible shortest trace and exits with status 1.

Expected trace shape:

```text
p1:request
p1:enter
p2:request
p2:enter
```

### Cyclic traffic light

```bash
cargo run -- run traffic-light
```

The model cycles `Red -> Green -> Yellow -> Red`. Visited-state detection proves the finite reachable graph has only three states and terminates.

## Tests

Run the same checks used by CI:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Coverage includes:

- reachable-state enumeration;
- invariant success and failure;
- shortest counterexample reconstruction;
- cyclic transition systems;
- multiple and duplicate initial states;
- deterministic checker/report output;
- violations in initial states;
- malformed model metadata and malformed transition labels.

## Model validation

Construction rejects empty model names, missing state variables, empty/duplicate state-variable names, missing initial states, missing invariants, and empty/duplicate invariant names. Exploration rejects transitions with empty action labels and propagates model-supplied transition-generation errors.

Validation is intentionally modest: this milestone does not attempt to prove that a user-supplied transition function is pure, finite, total, or deterministic.

## Limitations

Milestone 1 is deliberately small:

- explicit-state only; memory usage grows with the number of reachable states;
- safety invariants only;
- no LTL/CTL, fairness, liveness, Büchi automata, or temporal-property compiler;
- no SAT/SMT, BDDs, symbolic execution, theorem proving, or partial-order reduction;
- no state-space bounds or resource budgets yet;
- no large DSL or parser; models are ordinary Rust values/functions;
- no symmetry reduction, disk-backed state storage, parallel search, or distributed checking;
- determinism depends on the model returning successors in a deterministic order.

These omissions are intentional and belong to later milestones.

## Roadmap

A high-value next milestone is a **small typed modeling layer plus bounded exploration controls**, without jumping to symbolic solving. Candidate work:

1. explicit exploration limits with a distinct `inconclusive` result rather than conflating budget exhaustion with safety;
2. richer state/transition diagnostics and model validation;
3. a compact typed model-description API that still compiles into the same transition-system core;
4. additional classic protocols (Peterson, producer/consumer, leader-election fragments) with known-good and known-buggy variants;
5. property- and differential-style tests for BFS/trace invariants.

Only after the explicit-state semantics are well exercised should the project consider temporal logic, reduction techniques, or symbolic back ends.

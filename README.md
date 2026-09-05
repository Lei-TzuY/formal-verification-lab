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

The checker performs deterministic breadth-first search (BFS):

- initial states are enqueued in declaration order;
- duplicate initial states are ignored after their first occurrence;
- invariants are checked in declaration order when each state is dequeued;
- successor transitions are considered in the order returned by the model;
- previously visited states are not enqueued again;
- finite reachable state spaces terminate, including cyclic systems.

For each newly discovered state, the checker stores a predecessor node and the action used to reach it. If an invariant fails, those predecessor links reconstruct a counterexample from an initial state. Because exploration is BFS, the first reported violation has a **shortest number-of-transitions trace**. Ties among equally short traces are deterministic given deterministic model successor ordering.

### Milestone 2: bounded exploration with explicit incompleteness

`check_with_limits` adds optional bounds without weakening the meaning of `SAFE`:

- `max_states` bounds the number of unique states retained by the search;
- `max_transitions` bounds the number of transition edges examined;
- `max_depth` bounds the depth of newly discovered states from an initial state.

The result status is now one of:

- `SAFE` — the reachable graph was exhaustively explored within the supplied bounds and every checked invariant held;
- `VIOLATION` — a reachable state violated an invariant and a shortest counterexample is available;
- `INCONCLUSIVE` — a state, transition, or depth bound prevented exhaustive exploration.

A limit does **not** make a result inconclusive merely because its numeric value is reached. The checker only reports `INCONCLUSIVE` when the limit blocks work required to finish the proof. For example, a four-state model checked with `--max-states 4` can still be `SAFE` if no fifth state is reachable. Likewise, a depth boundary can still prove a closed cycle when every transition at the boundary points to an already visited state.

This distinction is a correctness invariant: resource exhaustion must never be reported as a proof of safety.

## Architecture

```text
src/model.rs      transition-system abstraction and model validation
src/checker.rs    deterministic BFS, resource bounds, visited-state tracking,
                  invariant checks, predecessor storage, shortest traces
src/examples.rs   executable teaching models
src/report.rs     deterministic text rendering (presentation layer)
src/main.rs       CLI and option parsing only; no transition-system semantics
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

The same proof can be run at exact resource bounds:

```bash
cargo run -- run counter --max-states 4 --max-transitions 3 --max-depth 3
```

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

If exploration is intentionally stopped before depth four, the same model is not called safe:

```bash
cargo run -- run mutex-bug --max-depth 3
```

That command prints `status: INCONCLUSIVE`, identifies the depth limit, and exits with status 3.

### Cyclic traffic light

```bash
cargo run -- run traffic-light
```

The model cycles `Red -> Green -> Yellow -> Red`. Visited-state detection proves the finite reachable graph has only three states and terminates. A depth limit of two is still sufficient because the boundary transition returns to the already visited red state.

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

- reachable-state enumeration;
- invariant success and failure;
- shortest counterexample reconstruction;
- cyclic transition systems;
- multiple and duplicate initial states;
- deterministic checker/report output;
- violations in initial states;
- malformed model metadata and malformed transition labels;
- state, transition, and depth exhaustion returning `INCONCLUSIVE`;
- exact-bound exhaustive proofs still returning `SAFE`;
- closed cycles at a depth boundary;
- sufficient limits preserving the same shortest counterexample.

CI additionally executes both the violation and bounded-inconclusive CLI paths and checks their exit codes and stable report markers.

## Model validation

Construction rejects empty model names, missing state variables, empty/duplicate state-variable names, missing initial states, missing invariants, and empty/duplicate invariant names. Exploration rejects transitions with empty action labels and propagates model-supplied transition-generation errors.

Validation is intentionally modest: the checker does not attempt to prove that a user-supplied transition function is pure, finite, total, or deterministic.

## Limitations

The current checker remains deliberately focused:

- explicit-state only; memory usage grows with the number of retained reachable states;
- resource limits bound semantic exploration but do not interrupt the user-provided successor function while it is constructing one state's transition vector;
- safety invariants only;
- no LTL/CTL, fairness, liveness, Büchi automata, or temporal-property compiler;
- no SAT/SMT, BDDs, symbolic execution, theorem proving, or partial-order reduction;
- no large DSL or parser; models are ordinary Rust values/functions;
- no symmetry reduction, disk-backed state storage, parallel search, or distributed checking;
- determinism depends on the model returning successors in a deterministic order.

These omissions are intentional and belong to later milestones.

## Roadmap

Milestones 1 and 2 establish trustworthy explicit-state safety semantics and explicit bounded-search outcomes. The next high-value architectural phase is a **typed modeling layer plus a known-correct concurrent protocol** that compiles into the existing transition-system core rather than bypassing it.

Candidate next work, in priority order:

1. a compact typed model builder that makes variables, initial states, transitions, and invariants easier to declare without introducing a large DSL;
2. Peterson's two-process mutual-exclusion algorithm as a known-good executable model, paired with a controlled buggy variant and checked safety properties;
3. richer exploration diagnostics such as frontier depth and per-action transition counts without changing proof semantics;
4. property- or differential-style tests for BFS and shortest-trace invariants;
5. only after the explicit-state layer is well exercised, carefully scoped temporal logic or reduction techniques.

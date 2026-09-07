# Project Status

This file records the current integration frontier. Historical capability detail remains in `README.md`; when the README roadmap lags an already-validated integration candidate, this status file is the current phase marker.

## Milestone 53 — textual multi-response temporal frontend

**Status: integration candidate complete.**

Milestone 53 adds a deterministic line-oriented textual frontend for conjunctions of named exact-action response obligations:

```text
response("class-a","request-a","grant-a")
response("class-b","request-b","grant-b")
```

The frontend validates non-empty property/clause/action names, rejects duplicate clause names, reports fail-closed line/UTF-8 byte-column parser diagnostics, supports the existing quoted-string escape set, and renders a deterministic canonical document.

`MultiResponseTemporalSpec` compiles directly to the sealed `MultiResponseProperty` semantics. The no-fair, combined weak/strong fairness, product-bounded, and staged model-before-product adapters delegate to the existing M11/M25/M28/M49 authorities; M53 adds no second traversal, response interpretation, fairness semantics, fairness-by-default behavior, wall-clock proof bound, or performance claim.

Executable evidence includes parser/canonical round trips, malformed-input and finite-terminal regressions, real declarative-model composition, exact differential equality for combined fairness and bounded/staged paths, and all 16 directed two-state graph masks × `4^4 = 256` exact-action assignments = **4,096** no-fair frontend/backend differential cases.

The M53 integration boundary is intentionally the Rust/textual specification frontend. A built-binary two-file command is not required by the M53 acceptance criteria and is promoted rather than folded into this review surface.

## Next frontier — Milestone 54: external multi-response temporal file CLI

M54 should expose the sealed M53 textual property documents together with declarative model files through the canonical `fvlab` binary, without duplicating fairness, limit, report, or response semantics.

Acceptance criteria:

- add one explicit external route for `<model-file> + <multi-response-property-file>` and parse each through the sealed M19 and M53 frontends;
- route the resulting `MultiResponseProperty` through the existing canonical multi-response orchestration/reporting instead of adding another verifier;
- preserve no-fair, weak-only, strong-only, canonical combined weak/strong fairness, product-only budgets, and staged model-before-product budgets;
- preserve exact clause identity, finite pending-terminal precedence, lasso evidence, cutoff accounting/provenance, and deterministic exit behavior (`0` satisfied, `7` direct multi-response violation, `3` inconclusive, `2` malformed input) unless an explicitly reviewed frontend-level exit contract replaces it;
- add built-binary two-file regressions for no-fair violation, combined-fair satisfaction, finite pending terminal, product/model cutoff reasons, malformed property input, and historical CLI compatibility;
- keep fairness external to the textual property grammar and make no new fairness, traversal, performance, or wall-clock-bound claim.

# Pre-registration additions

## 2026-11-24T12:35:41Z lexical-ladder proposal:f7a0048f67097636c62fb4f72cf017267e24532b3ff9606b6244b84c6f06581b

Hypothesis: on a seeded synthetic fixture of 400 claims where the contradicting
passage differs from the supporting one by a single negation or value token, the
token-overlap-0.8 rung accepts at least 0.80 of contradicting passages while
accepting at most 0.05 of unrelated passages, so the verbatim-token faithfulness
signal of section 3 cannot detect contradiction.

Metric: /ladder/rungs/overlap_0_8/contradicting_accept_rate (expected at least 0.80).

Falsifier: overlap_0_8 contradicting accept rate below 0.60, or unrelated accept
rate above 0.05 (fixture not discriminating).

The fixture is synthetic by construction (seed 7, generated in-crate); it
measures the signal's construction, not any real RAG system.

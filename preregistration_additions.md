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

## 2026-11-24T12:39:24Z negation-guard proposal:7412035f18af872cb1ed17fdcdba45d1aa0137c717c60057ff33f8ce250cd992

Hypothesis: on the seeded synthetic fixture (400 claims, seed 7) a negation-guard
rung (overlap 0.8 plus rejection when the passage's count of the token "not"
differs from the claim's) rejects every negation-type contradiction but accepts
every value-swap contradiction, so its overall contradicting accept rate equals
the value-swap share of the fixture, between 0.40 and 0.60, while supporting
accept stays 1.0 and unrelated accept stays at most 0.05.

Metric: /ladder/rungs/overlap_0_8_negation_guard/contradicting_accept_rate
(expected between 0.40 and 0.60, down from 1.0 at overlap_0_8).

Falsifier: negation-guard contradicting accept rate outside 0.40-0.60,
supporting accept below 1.0, or any negation-type contradiction accepted.

The fixture is synthetic by construction; it measures the signal's construction,
not any real RAG system.

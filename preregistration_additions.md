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

## 2026-11-24T12:43:14Z mixed-passage proposal:94f45988cfb7b3f37d7e9cfae790f6620717e699348c376f04eebf929ce4e658

Hypothesis: on the seeded synthetic fixture (400 claims, seed 7) a passage that
contains the claim verbatim and also a one-token contradicting variant of it
(supporting passage followed by contradicting passage) is accepted by every
lexical rung, including exact_substring and the negation-guard rung, so
mixed_accept_rate is 1.0 at each rung, while unrelated accept stays 0.0.

Metric: /ladder/rungs/exact_substring/mixed_accept_rate (expected 0.95-1.0;
also expected 1.0 at overlap_1_0 and overlap_0_8_negation_guard).

Falsifier: mixed_accept_rate below 0.95 at exact_substring, or unrelated accept
above 0.05, or supporting accept below 1.0. (Noted before running: the
negation-guard rung counts the token "not" over the whole passage, so it may
reject mixed passages of negation type; that part of the claim may fail.)

The fixture is synthetic by construction; it measures the signal's construction,
not any real RAG system.

## 2026-11-27T20:17:53Z withhold-conformal proposal:c0cc512cf0e6ca45377fd9da16f38a1f34f09a0b76c8cd7858c3b1e8f3320cc2

Hypothesis: on a seeded synthetic fixture (3000 questions, seed 7, equal thirds
sufficient/missing/misleading evidence; answer-correct probability 0.9/0.5/0.2
by condition; verifier score blind to the misleading condition, i.e. drawn from
the same distribution as sufficient), a split-conformal threshold set for an
accepted-wrong budget alpha=0.1 withholds answers of which a large share were
correct, so false_withhold_fraction at alpha 0.1 lies between 0.30 and 0.70 and
the missing-evidence condition contributes more false withholds than the
misleading condition.

Metric: /withhold/false_withhold_fraction_at_alpha_0_1 (expected 0.30-0.70).

Falsifier: false_withhold_fraction at alpha 0.1 outside 0.30-0.70, or the
misleading condition contributing as many or more false withholds than the
missing condition, or the realised accepted-wrong rate on the test split
exceeding alpha by more than 0.03.

Noted before running: the threshold rule requires (wrong+1)/(accepted+1) <= alpha
on the calibration half; if no threshold is feasible, everything is withheld
and the result reports that. The fixture is synthetic by construction.

## 2026-11-27T21:06:24Z sharedwrong proposal:e22bf20eb2524798ac9b9df8930afa2fba6520c08b242b313562010631af5574

Hypothesis: on a seeded synthetic fixture (3000 questions, seed 7, equal thirds
sufficient/missing/misleading evidence; answer-correct probability 0.9/0.5/0.2;
half of misleading questions have parametric memory agreeing with the passage;
a fixed support test passes 0.95 of sufficient and memory-agreeing misleading
cases but only 0.10 of missing and memory-disagreeing misleading cases), the
fraction of withheld answers that were correct lies between 0.30 and 0.55, and
memory-agreeing misleading questions supply more than half of all accepted
wrong answers.

Metric: /verifier/false_withhold_rate (expected 0.30-0.55; analytic expectation
about 0.41). Secondary: /verifier/agree_share_of_accepted_wrong (expected > 0.5).

Falsifier: false_withhold_rate outside 0.30-0.55, or agree_share_of_accepted_wrong
of 0.5 or less.

The fixture is synthetic by construction; it measures what the stated blind
spot implies, not any real verifier.

## 2026-11-27T22:32:45Z strictness proposal:04dd6e3f64ffa8f3a45481cd7b3fdb97003dd18293a7726a3afd97eca38bf063

Hypothesis: on a seeded synthetic fixture (3000 claims, seed 7, 70% supported;
verifier overlap score ~ clipped normal, mean 0.85 sd 0.12 for supported and
mean 0.45 sd 0.20 for unsupported), raising the verifier threshold from 0.50 to
0.95 (grid step 0.05; a claim is measured unsupported when its score is below
the threshold) raises the measured unsupported rate, and more than half of that
rise comes from false rejection of supported claims.

Metric: /strictness_sweep/false_rejection_share_of_rise (expected 0.50-0.95;
analytic expectation about 0.8).

Falsifier: false_rejection_share_of_rise of 0.50 or less, or measured
unsupported rate not increasing monotonically with threshold across the grid.

The fixture is synthetic by construction; it shows what the stated score
distributions imply, not any real verifier.

## 2026-11-27T23:14:02Z coverage proposal:6168274c6245a97459bd3a2feaa40539c4cb16b38226b001bd3dba05dd9ac3bd

Hypothesis: on the 3000-question synthetic fixture (seed 7) where
memory-agreeing misleading passages pass a support verifier, a conflict check
applied to accepted answers that fires on memory-agreeing misleading answers
with probability 0.8 and on correct accepted answers with probability 0.2
halves the accepted-wrong rate relative to the unchecked verifier only by
raising the false-withhold fraction (correct share of withheld answers) by at
least 0.05 over the unchecked verifier. Catch rate is swept on a 0.05 grid; the
metric is read at the smallest catch rate that halves the rate, or at catch 1.0
if none does (flagged by half_feasible_on_grid).

Metric: /coverage/false_withhold_increase_at_half_accepted_wrong (expected
0.05-0.25).

Falsifier: increase below 0.05, or the accepted-wrong rate not halved at the
0.8 catch rate (reported as stated_check_halves_accepted_wrong).

Synthetic fixture; it shows what the stated catch and false-fire rates imply,
not any real verifier.

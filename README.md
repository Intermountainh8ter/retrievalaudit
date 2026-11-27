# retrievalaudit

**Pre-registered RAG reproducibility auditor.** Pin the retriever,
corpus, generator, k, and reranker config; score retrieval
precision/recall@k and generator-vs-retrieved faithfulness with 95%
bootstrap CIs; emit one `Verdict ∈ {Scored, EmptyRetrieval, Unverified}`.

![status: initial scaffold](https://img.shields.io/badge/status-initial%20scaffold-lightgrey)
![language: Rust](https://img.shields.io/badge/language-Rust-blue)
![license: MIT](https://img.shields.io/badge/license-MIT-blue)

## Why

RAG rows are routinely non-comparable: the retriever's config
changed, the corpus gained documents, k went from 3 to 5, the
reranker's thresholds moved. Every one of those is a different row,
but almost no public RAG result commits all five. retrievalaudit is
the one-value row key (`manifest_sha256_hex`) plus the three signals
every reviewer wants to see.

## What is in this scaffold

- `RagManifest { retriever, corpus, generator, k, reranker_config_sha256_hex }`
- `precision_at_k`, `recall_at_k`, `faithfulness_score` — three per-query signals.
- `bootstrap_mean_ci` — 95% percentile CI, Pcg64Mcg, same recipe as the other crates.
- `audit(manifest, retrievals, truths, generations, …) -> AuditReport`
  — gates on §7 degenerate manifest and §5 always-empty retrieval
  before scoring.
- CLI: `canon`, `audit`, `version`. Exit codes 0/1/2 for
  Scored/EmptyRetrieval/Unverified.

## Lexical ladder (synthetic)

`retrievalaudit ladder` scores a seeded synthetic fixture (400 claims, seed 7;
no real RAG system) at four rungs. Results are in
`results/588-lexical-ladder.json`: the token-overlap-0.8 rung accepted
1.0 of contradicting passages (`/ladder/rungs/overlap_0_8/contradicting_accept_rate`)
and 0.0 of unrelated passages (`/ladder/rungs/overlap_0_8/unrelated_accept_rate`);
the exact-substring rung accepted 0.0 of contradicting passages. The fixture's
contradictions differ by one inserted negation or one swapped token by
construction, so this shows the signal's construction, not real-world rates.

### Negation guard

`results/589-negation-guard.json` adds a rung that keeps the 0.8 overlap test
and rejects a passage whose count of the token `not` differs from the claim's.
Its contradicting accept rate is 0.4975
(`/ladder/rungs/overlap_0_8_negation_guard/contradicting_accept_rate`), equal to
the fixture's value-swap share (`/ladder/value_swap_share`): negation-type
contradictions were rejected (`.../contradicting_negation_accept_rate`, 0.0) and
value-swap ones all accepted (`.../contradicting_value_swap_accept_rate`, 1.0).
The guard only sees the one token it was written for, on a synthetic fixture.

### Mixed passage

`results/590-mixed-passage.json` adds a passage holding the claim verbatim plus
its contradicting variant. The exact-substring rung accepted 1.0 of them
(`/ladder/rungs/exact_substring/mixed_accept_rate`), as did overlap 1.0. The
negation-guard rung accepted 0.4975
(`/ladder/rungs/overlap_0_8_negation_guard/mixed_accept_rate`), so that part of
the pre-registered expectation failed: its `not` count over the whole passage
rejects the negation-type mixes. Synthetic fixture.

### Withhold under a conformal budget

`results/700-withhold-conformal.json` simulates a verifier that sees missing
evidence but is blind to misleading evidence (3000 synthetic questions, seed 7,
half calibration, half test). No threshold met the accepted-wrong budget at
alpha 0.1 (`/withhold/alpha_0_1/feasible`, false), so every test answer was
withheld and the false-withhold fraction was 0.5407
(`/withhold/false_withhold_fraction_at_alpha_0_1`), inside the pre-registered
0.30-0.70 range. Missing evidence contributed 249 false withholds
(`/withhold/alpha_0_1/per_condition/missing/n_false_withhold`) against 104 for
misleading (`.../misleading/n_false_withhold`); sufficient evidence contributed
458 (`.../sufficient/n_false_withhold`). Synthetic fixture; the result mostly
shows that a blind spot makes the budget infeasible.

## What is openly deferred (§9)

- No live retriever / generator drivers. The scaffold scores
  committed JSONL fixtures.
- No semantic faithfulness grader. The pinned signal is verbatim
  token overlap; semantic grading is a separate pre-reg clause.
- No cross-manifest comparison. The pre-reg forbids it as "replication";
  a future `compare --unreproducible` subcommand would make
  cross-manifest commentary explicit.

## Related

Part of the [bench](https://github.com/Intermountainh8ter?tab=repositories)
suite of pre-registered audit crates.

## License

MIT.

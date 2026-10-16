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

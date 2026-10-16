# retrievalaudit pre-registration

Locked before the first live RAG audit result is committed.
Append-only.

## 1. Scope

The population under audit is **RAG pipelines**: a retriever reads a
query, pulls k documents from a corpus, and a generator reads the
query and those documents. For each pipeline we score retrieval
precision/recall@k against a ground-truth relevance set, and a
faithfulness score that measures how often the generator's output
uses the retrieved texts. Every scored row carries the pinned
manifest SHA — the same four facts (retriever, corpus, generator, k)
pinned the same way across all crates.

## 2. The five pinned fields

A `RagManifest` pins:

1. **Retriever** — `(name, config_sha256_hex)`. A different config
   SHA is a different retriever.
2. **Corpus** — `(name, sha256_hex, n_documents)`. A corpus that has
   gained a single document is a different corpus.
3. **Generator** — `(id, revision, chat_template_sha256_hex)`. Same
   shape as other bench crates.
4. **k** — the number of documents retrieved per query. A different
   k is a different row.
5. **Reranker config SHA** — optional. `None` means "no reranker";
   `Some("")` is `Unverified` (§7).

## 3. The three signals

- **Precision@k** — fraction of top-k retrieved documents that are
  in the query's relevant set.
- **Recall@k** — fraction of relevant documents that appear in the
  top-k. Defined as 1.0 on an empty relevance set; such rows are
  reported with their own column, not folded in.
- **Faithfulness** — fraction of the generator's whitespace-split
  tokens of length ≥ `min_token_len` that appear verbatim in the
  concatenated retrieved texts. Crude but committable; a semantic
  faithfulness grader is a separate pre-reg clause, not a swap-in.

## 4. Bootstrap CIs

Every reported mean carries a 95% percentile-bootstrap CI resampling
queries with replacement using Pcg64Mcg seeded from a caller-provided
seed. Pre-reg does not accept a point estimate.

## 5. Degenerate-baseline guard

A `RagManifest` whose retrieval returned 0 documents on every query
has nothing to be faithful to; the audit emits verdict
`EmptyRetrieval` and does not report faithfulness. This closes the
loophole where a broken retriever scored "faithful" because the
generator ignored the retrieval entirely.

## 6. Comparability rule

A row is comparable to another only when both carry the same
`manifest_sha256_hex`. Reports that compare rows across manifests
must say so explicitly — they are informative, not reproducible.

## 7. Degenerate-manifest guard

A manifest with any of these blank is `Unverified`, not a passing
manifest with defaults:

- `retriever.config_sha256_hex`
- `corpus.sha256_hex`
- `corpus.n_documents == 0`
- `generator.chat_template_sha256_hex`
- `k == 0`
- `reranker_config_sha256_hex == Some("")` (use `null` to mean "no
  reranker")

## 8. What this crate does NOT do

- Run retrievers or generators. retrievalaudit scores committed
  retrieval + generation fixtures.
- Judge whether a specific document is "relevant." The relevance set
  is caller-authored ground truth; this crate scores against it.
- Compute a semantic faithfulness score. The pinned signal is
  verbatim-token overlap; a semantic grader is a future clause.

## 9. Openly tracked gaps

- No live retriever adapter. A `retrievalaudit observe --retriever
  http://...` would spawn a retriever, run queries, and emit the
  retrievals JSONL — deferred.
- No live generator adapter. The faithfulness signal needs generator
  output; today the fixture provides it.
- No semantic faithfulness grader. The pinned signal is
  verbatim-token overlap; a NLI-based or embedding-based grader is a
  separate pre-reg clause.
- No cross-corpus comparison. A manifest change is a non-comparable
  row by §6, but a reviewer may still want to see "corpus vs
  corpus"; a future `compare` subcommand that explicitly tags the
  comparison as non-reproducible would land that.

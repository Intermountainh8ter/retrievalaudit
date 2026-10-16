//! Pre-registered RAG reproducibility auditor.
//!
//! A RAG pipeline is a tuple (retriever, corpus, generator, k). The
//! same question asked over the same corpus with a different retriever
//! or a different `k` yields different answers; so does the same
//! retrieval piped through a different generator. `RagManifest` pins
//! all five fields with hashes so a reviewer can tell a replicated row
//! from a lookalike. The library then computes retrieval
//! precision/recall@k against a ground-truth relevance set, and a
//! faithfulness score that measures how often the generator's output
//! actually uses the retrieved documents.
//!
//! See `docs/PREREGISTRATION.md` for the clauses this scaffold closes.

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Pre-reg §2 — the retriever identity. A different config SHA is a
/// different retriever, full stop.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Retriever {
    pub name: String,
    pub config_sha256_hex: String,
}

/// The corpus the retriever indexes. A different SHA is a different
/// corpus; a corpus that has gained even one document is a different
/// corpus.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Corpus {
    pub name: String,
    pub sha256_hex: String,
    pub n_documents: usize,
}

/// The generator that reads the retrieved documents.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Generator {
    pub id: String,
    pub revision: String,
    /// Chat template SHA at run time. Pre-reg §2 point 4.
    pub chat_template_sha256_hex: String,
}

/// The full pinned manifest for a RAG row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RagManifest {
    pub retriever: Retriever,
    pub corpus: Corpus,
    pub generator: Generator,
    pub k: usize,
    /// Optional reranker config SHA. `None` means "no reranker"; an
    /// empty string here is `Unverified` (pre-reg §7).
    #[serde(default)]
    pub reranker_config_sha256_hex: Option<String>,
}

/// One query's retrieval output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub query_id: String,
    pub retrieved_doc_ids: Vec<String>,
}

/// One query's ground-truth relevance set.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroundTruth {
    pub query_id: String,
    pub relevant_doc_ids: Vec<String>,
}

/// Precision@k: of the top k retrieved documents, what fraction are
/// in the ground-truth relevant set. `k=0` or an empty retrieval
/// returns 0.0.
pub fn precision_at_k(retrieval: &RetrievalResult, truth: &GroundTruth, k: usize) -> f64 {
    if k == 0 || retrieval.retrieved_doc_ids.is_empty() {
        return 0.0;
    }
    let top: Vec<&String> = retrieval.retrieved_doc_ids.iter().take(k).collect();
    let hits = top
        .iter()
        .filter(|d| truth.relevant_doc_ids.iter().any(|r| r == **d))
        .count();
    hits as f64 / top.len() as f64
}

/// Recall@k: of the ground-truth relevant documents, what fraction
/// appear in the top k retrieved. If the relevant set is empty,
/// returns 1.0 — nothing to miss. Pre-reg §3 is explicit: the "empty
/// relevance set" row is reported with its own column, not folded in.
pub fn recall_at_k(retrieval: &RetrievalResult, truth: &GroundTruth, k: usize) -> f64 {
    if truth.relevant_doc_ids.is_empty() {
        return 1.0;
    }
    let top: Vec<&String> = retrieval.retrieved_doc_ids.iter().take(k).collect();
    let hits = truth
        .relevant_doc_ids
        .iter()
        .filter(|r| top.iter().any(|d| **d == **r))
        .count();
    hits as f64 / truth.relevant_doc_ids.len() as f64
}

/// One query's generation output alongside the retrieval it saw.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationResult {
    pub query_id: String,
    pub retrieved_doc_ids: Vec<String>,
    pub retrieved_doc_texts: Vec<String>,
    pub generated_text: String,
}

/// Faithfulness score: fraction of the generator's whitespace-split
/// tokens of length >= `min_token_len` that appear verbatim anywhere
/// in the concatenated retrieved texts. A proxy for "the generation
/// used the retrieval" — crude but committable. Pre-reg §3 says this
/// is the pinned signal; a semantic faithfulness grader is a separate
/// pre-reg clause, not a swap-in.
pub fn faithfulness_score(result: &GenerationResult, min_token_len: usize) -> f64 {
    let haystack = result.retrieved_doc_texts.join(" ").to_lowercase();
    let tokens: Vec<String> = result
        .generated_text
        .to_lowercase()
        .split_whitespace()
        .filter(|t| t.len() >= min_token_len)
        .map(|t| t.to_string())
        .collect();
    if tokens.is_empty() {
        return 0.0;
    }
    let hits = tokens.iter().filter(|t| haystack.contains(t.as_str())).count();
    hits as f64 / tokens.len() as f64
}

/// Pre-reg §5 degenerate-baseline marker: a RAG row whose retrieval
/// returned 0 documents on every query. Such a row has nothing to be
/// faithful to; the pre-reg forbids reporting faithfulness for it.
pub fn always_empty_retrieval(results: &[RetrievalResult]) -> bool {
    !results.is_empty() && results.iter().all(|r| r.retrieved_doc_ids.is_empty())
}

/// Mean + 95% percentile-bootstrap CI across a slice of per-query
/// scores. Pre-reg §4 pins the method (percentile method, pcg64mcg).
pub fn bootstrap_mean_ci(values: &[f64], iters: usize, seed: u64) -> (f64, f64, f64) {
    assert!(!values.is_empty());
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let mut means = Vec::with_capacity(iters);
    let n = values.len();
    for _ in 0..iters {
        let mut sum = 0.0;
        for _ in 0..n {
            let k = (rand::Rng::gen::<u32>(&mut rng) as usize) % n;
            sum += values[k];
        }
        means.push(sum / n as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = values.iter().sum::<f64>() / n as f64;
    let lo = means[(iters as f64 * 0.025).floor() as usize];
    let hi = means[((iters as f64 * 0.975).ceil() as usize).min(iters - 1)];
    (mean, lo, hi)
}

/// Canonical SHA-256 of a manifest. Same recipe as the other bench
/// crates: sorted-keys, whitespace-free JSON.
pub fn manifest_sha256_hex(m: &RagManifest) -> String {
    let v = serde_json::to_value(m).expect("manifest serializes");
    let mut out = Vec::new();
    write_canonical(&v, &mut out);
    let d = Sha256::digest(&out);
    let mut s = String::with_capacity(d.len() * 2);
    for b in d {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn write_canonical(v: &serde_json::Value, buf: &mut Vec<u8>) {
    match v {
        serde_json::Value::Object(map) => {
            buf.push(b'{');
            let mut keys: Vec<&std::string::String> = map.keys().collect();
            keys.sort();
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                buf.extend_from_slice(serde_json::to_string(k).unwrap().as_bytes());
                buf.push(b':');
                write_canonical(&map[k.as_str()], buf);
            }
            buf.push(b'}');
        }
        serde_json::Value::Array(arr) => {
            buf.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical(item, buf);
            }
            buf.push(b']');
        }
        other => buf.extend_from_slice(serde_json::to_string(other).unwrap().as_bytes()),
    }
}

/// Pre-reg §7 degenerate check for the manifest itself: no blank
/// hashes, k > 0, corpus has at least one document.
pub fn is_degenerate(m: &RagManifest) -> Option<String> {
    let mut reasons = Vec::new();
    if m.retriever.config_sha256_hex.trim().is_empty() {
        reasons.push("retriever.config_sha256_hex is empty");
    }
    if m.corpus.sha256_hex.trim().is_empty() {
        reasons.push("corpus.sha256_hex is empty");
    }
    if m.corpus.n_documents == 0 {
        reasons.push("corpus.n_documents is 0");
    }
    if m.generator.chat_template_sha256_hex.trim().is_empty() {
        reasons.push("generator.chat_template_sha256_hex is empty");
    }
    if m.k == 0 {
        reasons.push("k is 0");
    }
    if let Some(s) = &m.reranker_config_sha256_hex {
        if s.trim().is_empty() {
            reasons.push("reranker_config_sha256_hex is Some(empty) — use null to mean no reranker");
        }
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join("; "))
    }
}

/// Full audit report for a RAG row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuditReport {
    pub manifest: RagManifest,
    pub manifest_sha256_hex: String,
    pub n_queries: usize,
    pub precision_mean: f64,
    pub precision_ci_lo: f64,
    pub precision_ci_hi: f64,
    pub recall_mean: f64,
    pub recall_ci_lo: f64,
    pub recall_ci_hi: f64,
    pub faithfulness_mean: f64,
    pub faithfulness_ci_lo: f64,
    pub faithfulness_ci_hi: f64,
    pub always_empty_retrieval: bool,
    pub verdict: Verdict,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verdict {
    /// Pre-reg §7 blocks the row: manifest is degenerate.
    Unverified(String),
    /// Pre-reg §5 blocks the row: retrieval returned nothing on every
    /// query (faithfulness is undefined).
    EmptyRetrieval,
    /// Scored cleanly.
    Scored,
}

/// Compute the full audit from pinned manifest + results. Pre-reg §4:
/// gates apply before scoring.
#[allow(clippy::too_many_arguments)]
pub fn audit(
    manifest: &RagManifest,
    retrievals: &[RetrievalResult],
    truths: &[GroundTruth],
    generations: &[GenerationResult],
    min_token_len: usize,
    boot_iters: usize,
    boot_seed: u64,
) -> AuditReport {
    let manifest_sha = manifest_sha256_hex(manifest);
    if let Some(r) = is_degenerate(manifest) {
        return empty_report(manifest.clone(), manifest_sha, Verdict::Unverified(r));
    }
    if always_empty_retrieval(retrievals) {
        return empty_report(manifest.clone(), manifest_sha, Verdict::EmptyRetrieval);
    }
    let precision: Vec<f64> = retrievals
        .iter()
        .zip(truths.iter())
        .map(|(r, t)| precision_at_k(r, t, manifest.k))
        .collect();
    let recall: Vec<f64> = retrievals
        .iter()
        .zip(truths.iter())
        .map(|(r, t)| recall_at_k(r, t, manifest.k))
        .collect();
    let faithfulness: Vec<f64> = generations
        .iter()
        .map(|g| faithfulness_score(g, min_token_len))
        .collect();
    let (pm, pl, ph) = if precision.is_empty() {
        (f64::NAN, f64::NAN, f64::NAN)
    } else {
        bootstrap_mean_ci(&precision, boot_iters, boot_seed)
    };
    let (rm, rl, rh) = if recall.is_empty() {
        (f64::NAN, f64::NAN, f64::NAN)
    } else {
        bootstrap_mean_ci(&recall, boot_iters, boot_seed)
    };
    let (fm, fl, fh) = if faithfulness.is_empty() {
        (f64::NAN, f64::NAN, f64::NAN)
    } else {
        bootstrap_mean_ci(&faithfulness, boot_iters, boot_seed)
    };
    AuditReport {
        manifest: manifest.clone(),
        manifest_sha256_hex: manifest_sha,
        n_queries: retrievals.len(),
        precision_mean: pm,
        precision_ci_lo: pl,
        precision_ci_hi: ph,
        recall_mean: rm,
        recall_ci_lo: rl,
        recall_ci_hi: rh,
        faithfulness_mean: fm,
        faithfulness_ci_lo: fl,
        faithfulness_ci_hi: fh,
        always_empty_retrieval: false,
        verdict: Verdict::Scored,
    }
}

fn empty_report(manifest: RagManifest, manifest_sha: String, verdict: Verdict) -> AuditReport {
    AuditReport {
        manifest,
        manifest_sha256_hex: manifest_sha,
        n_queries: 0,
        precision_mean: f64::NAN,
        precision_ci_lo: f64::NAN,
        precision_ci_hi: f64::NAN,
        recall_mean: f64::NAN,
        recall_ci_lo: f64::NAN,
        recall_ci_hi: f64::NAN,
        faithfulness_mean: f64::NAN,
        faithfulness_ci_lo: f64::NAN,
        faithfulness_ci_hi: f64::NAN,
        always_empty_retrieval: matches!(verdict, Verdict::EmptyRetrieval),
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> RagManifest {
        RagManifest {
            retriever: Retriever {
                name: "bge-m3".into(),
                config_sha256_hex: "a".repeat(64),
            },
            corpus: Corpus {
                name: "wiki2024".into(),
                sha256_hex: "b".repeat(64),
                n_documents: 10_000,
            },
            generator: Generator {
                id: "org/model".into(),
                revision: "r1".into(),
                chat_template_sha256_hex: "c".repeat(64),
            },
            k: 5,
            reranker_config_sha256_hex: None,
        }
    }

    #[test]
    fn precision_at_k_matches_hand_calc() {
        let r = RetrievalResult {
            query_id: "q".into(),
            retrieved_doc_ids: vec!["d1".into(), "d2".into(), "d3".into(), "d4".into()],
        };
        let t = GroundTruth {
            query_id: "q".into(),
            relevant_doc_ids: vec!["d2".into(), "d3".into(), "d99".into()],
        };
        // top 3: d1, d2, d3 → 2 hits of 3.
        assert!((precision_at_k(&r, &t, 3) - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn recall_at_k_counts_hits_against_relevant_set() {
        let r = RetrievalResult {
            query_id: "q".into(),
            retrieved_doc_ids: vec!["d1".into(), "d2".into(), "d3".into()],
        };
        let t = GroundTruth {
            query_id: "q".into(),
            relevant_doc_ids: vec!["d2".into(), "d5".into()],
        };
        // relevant {d2, d5}, retrieved top-3 {d1,d2,d3} → 1 of 2.
        assert_eq!(recall_at_k(&r, &t, 3), 0.5);
    }

    #[test]
    fn recall_defined_as_one_on_empty_relevance_set() {
        let r = RetrievalResult {
            query_id: "q".into(),
            retrieved_doc_ids: vec!["d1".into()],
        };
        let t = GroundTruth {
            query_id: "q".into(),
            relevant_doc_ids: vec![],
        };
        assert_eq!(recall_at_k(&r, &t, 3), 1.0);
    }

    #[test]
    fn faithfulness_counts_retrieved_long_tokens_only() {
        let g = GenerationResult {
            query_id: "q".into(),
            retrieved_doc_ids: vec!["d1".into()],
            retrieved_doc_texts: vec!["the mitochondria is the powerhouse of the cell".into()],
            generated_text: "mitochondria power the cell for brief periods".into(),
        };
        // min_token_len=5: generated long tokens = {mitochondria, power, brief, periods}.
        // In the haystack: "mitochondria" ✓, "power" is substring of
        // "powerhouse" ✓, "brief" ✗, "periods" ✗ → 2/4.
        let s = faithfulness_score(&g, 5);
        assert!((s - 0.5).abs() < 1e-6, "s={s}");
    }

    #[test]
    fn always_empty_retrieval_is_only_true_when_every_row_is_empty() {
        let empty = vec![
            RetrievalResult {
                query_id: "q1".into(),
                retrieved_doc_ids: vec![],
            },
            RetrievalResult {
                query_id: "q2".into(),
                retrieved_doc_ids: vec![],
            },
        ];
        assert!(always_empty_retrieval(&empty));
        let one_nonempty = vec![
            RetrievalResult {
                query_id: "q1".into(),
                retrieved_doc_ids: vec![],
            },
            RetrievalResult {
                query_id: "q2".into(),
                retrieved_doc_ids: vec!["d1".into()],
            },
        ];
        assert!(!always_empty_retrieval(&one_nonempty));
        // An empty vec is NOT a degenerate-baseline — there are no
        // queries to be empty on.
        assert!(!always_empty_retrieval(&[]));
    }

    #[test]
    fn is_degenerate_names_each_missing_pin() {
        let mut m = manifest();
        m.retriever.config_sha256_hex = "".into();
        m.k = 0;
        let r = is_degenerate(&m).expect("degenerate");
        assert!(r.contains("retriever.config_sha256_hex"));
        assert!(r.contains("k is 0"));
    }

    #[test]
    fn is_degenerate_rejects_empty_reranker_string() {
        let mut m = manifest();
        m.reranker_config_sha256_hex = Some("".into());
        let r = is_degenerate(&m).expect("degenerate");
        assert!(r.contains("reranker"));
    }

    #[test]
    fn manifest_hash_is_lowercase_hex_sha256() {
        let h = manifest_sha256_hex(&manifest());
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn manifest_hash_changes_when_any_pinned_field_changes() {
        let base = manifest();
        let base_h = manifest_sha256_hex(&base);
        let mut m2 = base.clone();
        m2.k = 10;
        assert_ne!(manifest_sha256_hex(&m2), base_h);
    }

    #[test]
    fn audit_blocks_on_degenerate_manifest() {
        let mut m = manifest();
        m.corpus.n_documents = 0;
        let r = audit(&m, &[], &[], &[], 5, 100, 1);
        assert!(matches!(r.verdict, Verdict::Unverified(_)));
    }

    #[test]
    fn audit_blocks_on_always_empty_retrieval() {
        let m = manifest();
        let retrievals = vec![RetrievalResult {
            query_id: "q".into(),
            retrieved_doc_ids: vec![],
        }];
        let truths = vec![GroundTruth {
            query_id: "q".into(),
            relevant_doc_ids: vec!["d1".into()],
        }];
        let r = audit(&m, &retrievals, &truths, &[], 5, 100, 1);
        assert_eq!(r.verdict, Verdict::EmptyRetrieval);
        assert!(r.always_empty_retrieval);
    }

    #[test]
    fn audit_scores_cleanly_on_valid_input() {
        let m = manifest();
        let retrievals: Vec<_> = (0..20)
            .map(|i| RetrievalResult {
                query_id: format!("q{i}"),
                retrieved_doc_ids: vec!["d1".into(), "d2".into(), "d3".into()],
            })
            .collect();
        let truths: Vec<_> = (0..20)
            .map(|i| GroundTruth {
                query_id: format!("q{i}"),
                relevant_doc_ids: vec!["d1".into()],
            })
            .collect();
        let generations: Vec<_> = (0..20)
            .map(|i| GenerationResult {
                query_id: format!("q{i}"),
                retrieved_doc_ids: vec!["d1".into()],
                retrieved_doc_texts: vec!["answer is forty two".into()],
                generated_text: "answer forty".into(),
            })
            .collect();
        let r = audit(&m, &retrievals, &truths, &generations, 4, 100, 1);
        assert_eq!(r.verdict, Verdict::Scored);
        assert_eq!(r.n_queries, 20);
        assert!(r.precision_mean > 0.0);
    }

    #[test]
    fn audit_round_trips_json() {
        let m = manifest();
        let retrievals = vec![RetrievalResult {
            query_id: "q".into(),
            retrieved_doc_ids: vec!["d1".into()],
        }];
        let truths = vec![GroundTruth {
            query_id: "q".into(),
            relevant_doc_ids: vec!["d1".into()],
        }];
        let generations = vec![GenerationResult {
            query_id: "q".into(),
            retrieved_doc_ids: vec!["d1".into()],
            retrieved_doc_texts: vec!["text".into()],
            generated_text: "text".into(),
        }];
        let r = audit(&m, &retrievals, &truths, &generations, 3, 50, 1);
        let s = serde_json::to_string(&r).unwrap();
        let r2: AuditReport = serde_json::from_str(&s).unwrap();
        assert_eq!(r.verdict, r2.verdict);
    }
}

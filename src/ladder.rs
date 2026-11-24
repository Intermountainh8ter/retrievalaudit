//! Model-free lexical strictness ladder on a seeded synthetic fixture.
//!
//! Each claim is paired with a supporting passage, a contradicting passage
//! (one negation token inserted, or one value token swapped) and an unrelated
//! passage. Rungs: exact substring and claim-token overlap >= 0.5, 0.8, 1.0.

use crate::bootstrap_mean_ci;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde_json::{json, Value};

pub struct Item {
    pub claim: Vec<String>,
    pub supporting: Vec<String>,
    pub contradicting: Vec<String>,
    pub unrelated: Vec<String>,
    pub negation_type: bool,
}

fn word(prefix: &str, i: usize) -> String {
    format!("{prefix}{i}")
}

pub fn build_fixture(seed: u64, n: usize) -> Vec<Item> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        let len = rng.gen_range(8..=12usize);
        let claim: Vec<String> = (0..len).map(|_| word("c", rng.gen_range(0..5000))).collect();
        let pad = |rng: &mut Pcg64Mcg| -> Vec<String> {
            (0..rng.gen_range(3..=6usize)).map(|_| word("f", rng.gen_range(0..5000))).collect()
        };
        let mut supporting = pad(&mut rng);
        supporting.extend(claim.iter().cloned());
        supporting.extend(pad(&mut rng));
        let mut contra_claim = claim.clone();
        let pos = rng.gen_range(1..len);
        let negation_type = rng.gen::<bool>();
        if negation_type {
            contra_claim.insert(pos, "not".to_string());
        } else {
            contra_claim[pos] = word("v", rng.gen_range(0..5000));
        }
        let mut contradicting = pad(&mut rng);
        contradicting.extend(contra_claim);
        contradicting.extend(pad(&mut rng));
        let unrelated: Vec<String> =
            (0..len + 8).map(|_| word("u", rng.gen_range(0..5000))).collect();
        items.push(Item { claim, supporting, contradicting, unrelated, negation_type });
    }
    items
}

/// Supporting passage followed by the contradicting one: contains the claim verbatim
/// and also a one-token contradicting variant of it.
fn mixed_passage(it: &Item) -> Vec<String> {
    let mut p = it.supporting.clone();
    p.extend(it.contradicting.iter().cloned());
    p
}

fn contains_run(passage: &[String], claim: &[String]) -> bool {
    passage.windows(claim.len()).any(|w| w == claim)
}

fn overlap(passage: &[String], claim: &[String]) -> f64 {
    let hit = claim.iter().filter(|t| passage.contains(t)).count();
    hit as f64 / claim.len() as f64
}

fn not_count(tokens: &[String]) -> usize {
    tokens.iter().filter(|t| t.as_str() == "not").count()
}

#[derive(Clone, Copy)]
enum Rung {
    Exact,
    Overlap(f64),
    NegationGuard(f64),
}

fn accepts(r: Rung, passage: &[String], claim: &[String]) -> f64 {
    let ok = match r {
        Rung::Exact => contains_run(passage, claim),
        Rung::Overlap(t) => overlap(passage, claim) >= t - 1e-12,
        Rung::NegationGuard(t) => {
            overlap(passage, claim) >= t - 1e-12 && not_count(passage) == not_count(claim)
        }
    };
    if ok { 1.0 } else { 0.0 }
}

fn rate(vals: &[f64], boot: usize, seed: u64) -> Value {
    let (mean, lo, hi) = bootstrap_mean_ci(vals, boot, seed);
    json!({ "rate": mean, "ci95_lo": lo, "ci95_hi": hi })
}

/// Returns the `ladder` object; contradicting_accept_rate etc. are flat leaves.
pub fn run_ladder(seed: u64, n: usize, boot: usize) -> Value {
    let items = build_fixture(seed, n);
    let rungs: [(&str, Rung); 5] = [
        ("exact_substring", Rung::Exact),
        ("overlap_0_5", Rung::Overlap(0.5)),
        ("overlap_0_8", Rung::Overlap(0.8)),
        ("overlap_1_0", Rung::Overlap(1.0)),
        ("overlap_0_8_negation_guard", Rung::NegationGuard(0.8)),
    ];
    let mut out = serde_json::Map::new();
    for (name, r) in rungs {
        let col = |f: fn(&Item) -> &Vec<String>| -> Vec<f64> {
            items.iter().map(|it| accepts(r, f(it), &it.claim)).collect()
        };
        let s = col(|i| &i.supporting);
        let c = col(|i| &i.contradicting);
        let u = col(|i| &i.unrelated);
        let m: Vec<f64> = items
            .iter()
            .map(|it| accepts(r, &mixed_passage(it), &it.claim))
            .collect();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let mut o = serde_json::Map::new();
        o.insert("supporting_accept_rate".into(), json!(mean(&s)));
        o.insert("contradicting_accept_rate".into(), json!(mean(&c)));
        o.insert("unrelated_accept_rate".into(), json!(mean(&u)));
        let split = |neg: bool| -> f64 {
            let v: Vec<f64> = items
                .iter()
                .filter(|it| it.negation_type == neg)
                .map(|it| accepts(r, &it.contradicting, &it.claim))
                .collect();
            if v.is_empty() { 0.0 } else { mean(&v) }
        };
        o.insert("contradicting_negation_accept_rate".into(), json!(split(true)));
        o.insert("contradicting_value_swap_accept_rate".into(), json!(split(false)));
        o.insert("mixed_accept_rate".into(), json!(mean(&m)));
        o.insert("supporting_ci".into(), rate(&s, boot, seed));
        o.insert("contradicting_ci".into(), rate(&c, boot, seed));
        o.insert("unrelated_ci".into(), rate(&u, boot, seed));
        out.insert(name.into(), Value::Object(o));
    }
    let value_swap_share =
        items.iter().filter(|it| !it.negation_type).count() as f64 / n as f64;
    json!({ "value_swap_share": value_swap_share, "n_claims": n, "seed": seed, "boot": boot, "synthetic": true, "rungs": out })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_passage_accepted_by_exact_and_overlap() {
        let v = run_ladder(7, 50, 20);
        for r in ["exact_substring", "overlap_1_0", "overlap_0_8"] {
            assert_eq!(v["rungs"][r]["mixed_accept_rate"].as_f64().unwrap(), 1.0);
        }
        assert_eq!(v["rungs"]["exact_substring"]["unrelated_accept_rate"].as_f64().unwrap(), 0.0);
    }

    #[test]
    fn deterministic() {
        assert_eq!(run_ladder(7, 50, 100), run_ladder(7, 50, 100));
    }

    #[test]
    fn supporting_always_accepted_and_exact_rejects_contradiction() {
        let v = run_ladder(7, 100, 100);
        for k in ["exact_substring", "overlap_0_5", "overlap_0_8", "overlap_1_0"] {
            assert_eq!(v["rungs"][k]["supporting_accept_rate"], 1.0);
        }
        assert_eq!(v["rungs"]["exact_substring"]["contradicting_accept_rate"], 0.0);
        assert_eq!(v["rungs"]["overlap_0_8"]["unrelated_accept_rate"], 0.0);
    }

    #[test]
    fn negation_guard_rejects_negation_keeps_value_swap() {
        let v = run_ladder(7, 400, 100);
        let g = &v["rungs"]["overlap_0_8_negation_guard"];
        assert_eq!(g["contradicting_negation_accept_rate"], 0.0);
        assert_eq!(g["contradicting_value_swap_accept_rate"], 1.0);
        assert_eq!(g["supporting_accept_rate"], 1.0);
        assert_eq!(g["contradicting_accept_rate"], v["value_swap_share"]);
    }

    #[test]
    fn not_count_differs_on_negation() {
        let c: Vec<String> = vec!["a".into(), "b".into()];
        let p: Vec<String> = vec!["a".into(), "not".into(), "b".into()];
        assert_ne!(not_count(&c), not_count(&p));
    }

    #[test]
    fn overlap_cannot_see_negation() {
        let claim: Vec<String> = "a b c d e f g h".split(' ').map(String::from).collect();
        let mut p = claim.clone();
        p.insert(3, "not".into());
        assert_eq!(overlap(&p, &claim), 1.0);
        assert!(!contains_run(&p, &claim));
    }
}

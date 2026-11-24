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
        if rng.gen::<bool>() {
            contra_claim.insert(pos, "not".to_string());
        } else {
            contra_claim[pos] = word("v", rng.gen_range(0..5000));
        }
        let mut contradicting = pad(&mut rng);
        contradicting.extend(contra_claim);
        contradicting.extend(pad(&mut rng));
        let unrelated: Vec<String> =
            (0..len + 8).map(|_| word("u", rng.gen_range(0..5000))).collect();
        items.push(Item { claim, supporting, contradicting, unrelated });
    }
    items
}

fn contains_run(passage: &[String], claim: &[String]) -> bool {
    passage.windows(claim.len()).any(|w| w == claim)
}

fn overlap(passage: &[String], claim: &[String]) -> f64 {
    let hit = claim.iter().filter(|t| passage.contains(t)).count();
    hit as f64 / claim.len() as f64
}

#[derive(Clone, Copy)]
enum Rung {
    Exact,
    Overlap(f64),
}

fn accepts(r: Rung, passage: &[String], claim: &[String]) -> f64 {
    let ok = match r {
        Rung::Exact => contains_run(passage, claim),
        Rung::Overlap(t) => overlap(passage, claim) >= t - 1e-12,
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
    let rungs: [(&str, Rung); 4] = [
        ("exact_substring", Rung::Exact),
        ("overlap_0_5", Rung::Overlap(0.5)),
        ("overlap_0_8", Rung::Overlap(0.8)),
        ("overlap_1_0", Rung::Overlap(1.0)),
    ];
    let mut out = serde_json::Map::new();
    for (name, r) in rungs {
        let col = |f: fn(&Item) -> &Vec<String>| -> Vec<f64> {
            items.iter().map(|it| accepts(r, f(it), &it.claim)).collect()
        };
        let s = col(|i| &i.supporting);
        let c = col(|i| &i.contradicting);
        let u = col(|i| &i.unrelated);
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let mut o = serde_json::Map::new();
        o.insert("supporting_accept_rate".into(), json!(mean(&s)));
        o.insert("contradicting_accept_rate".into(), json!(mean(&c)));
        o.insert("unrelated_accept_rate".into(), json!(mean(&u)));
        o.insert("supporting_ci".into(), rate(&s, boot, seed));
        o.insert("contradicting_ci".into(), rate(&c, boot, seed));
        o.insert("unrelated_ci".into(), rate(&u, boot, seed));
        out.insert(name.into(), Value::Object(o));
    }
    json!({ "n_claims": n, "seed": seed, "boot": boot, "synthetic": true, "rungs": out })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn overlap_cannot_see_negation() {
        let claim: Vec<String> = "a b c d e f g h".split(' ').map(String::from).collect();
        let mut p = claim.clone();
        p.insert(3, "not".into());
        assert_eq!(overlap(&p, &claim), 1.0);
        assert!(!contains_run(&p, &claim));
    }
}

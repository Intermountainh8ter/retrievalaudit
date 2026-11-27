//! Model-free withhold simulation: a citation verifier with a misleading-context
//! blind spot, a split-conformal threshold at accepted-wrong budget alpha, and the
//! fraction of withheld answers that were correct, per evidence condition.

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde_json::{json, Value};

const CONDITIONS: [&str; 3] = ["sufficient", "missing", "misleading"];
const P_CORRECT: [f64; 3] = [0.9, 0.5, 0.2];
const ALPHAS: [(&str, f64); 3] = [("alpha_0_05", 0.05), ("alpha_0_1", 0.1), ("alpha_0_3", 0.3)];

#[derive(Clone, Copy)]
pub struct Q {
    pub cond: usize,
    pub correct: bool,
    pub score: f64,
}

fn normal(rng: &mut Pcg64Mcg) -> f64 {
    let s: f64 = (0..12).map(|_| rng.gen::<f64>()).sum();
    s - 6.0
}

pub fn build_fixture(seed: u64, n: usize) -> Vec<Q> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    (0..n)
        .map(|i| {
            let cond = i % 3;
            let correct = rng.gen::<f64>() < P_CORRECT[cond];
            // verifier sees missing evidence (low mean) but not misleading (same as sufficient)
            let mean = if cond == 1 { -1.0 } else { 1.0 };
            Q { cond, correct, score: mean + normal(&mut rng) }
        })
        .collect()
}

/// Lowest threshold t with (wrong+1)/(accepted+1) <= alpha on the calibration set
/// where accepted = score >= t; None if no threshold is feasible.
pub fn calibrate(cal: &[Q], alpha: f64) -> Option<f64> {
    let mut s: Vec<Q> = cal.to_vec();
    s.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let (mut acc, mut wrong) = (0usize, 0usize);
    let mut best = None;
    for q in &s {
        acc += 1;
        if !q.correct {
            wrong += 1;
        }
        if (wrong as f64 + 1.0) / (acc as f64 + 1.0) <= alpha {
            best = Some(q.score);
        }
    }
    best
}

fn evaluate(test: &[Q], thr: Option<f64>) -> Value {
    let accepted = |q: &Q| thr.map_or(false, |t| q.score >= t);
    let n_acc = test.iter().filter(|q| accepted(q)).count();
    let acc_wrong = test.iter().filter(|q| accepted(q) && !q.correct).count();
    let withheld: Vec<&Q> = test.iter().filter(|q| !accepted(q)).collect();
    let fw = withheld.iter().filter(|q| q.correct).count();
    let mut per = serde_json::Map::new();
    for (c, name) in CONDITIONS.iter().enumerate() {
        let w: Vec<&&Q> = withheld.iter().filter(|q| q.cond == c).collect();
        let wc = w.iter().filter(|q| q.correct).count();
        per.insert(
            name.to_string(),
            json!({
                "n_withheld": w.len(),
                "n_false_withhold": wc,
                "false_withhold_fraction": if w.is_empty() { Value::Null } else { json!(wc as f64 / w.len() as f64) },
                "false_withhold_share_of_all": if fw == 0 { Value::Null } else { json!(wc as f64 / fw as f64) },
            }),
        );
    }
    json!({
        "feasible": thr.is_some(),
        "threshold": thr,
        "n_accepted": n_acc,
        "n_withheld": withheld.len(),
        "realised_accepted_wrong_rate": if n_acc == 0 { Value::Null } else { json!(acc_wrong as f64 / n_acc as f64) },
        "false_withhold_fraction": if withheld.is_empty() { Value::Null } else { json!(fw as f64 / withheld.len() as f64) },
        "per_condition": per,
    })
}

pub fn run_withhold(seed: u64, n: usize) -> Value {
    let mut qs = build_fixture(seed, n);
    let mut rng = Pcg64Mcg::seed_from_u64(seed ^ 0x5eed);
    for i in (1..qs.len()).rev() {
        qs.swap(i, rng.gen_range(0..=i));
    }
    let half = qs.len() / 2;
    let (cal, test) = qs.split_at(half);
    let mut out = serde_json::Map::new();
    for (name, a) in ALPHAS {
        let thr = calibrate(cal, a);
        let mut v = evaluate(test, thr);
        if name == "alpha_0_1" {
            out.insert("false_withhold_fraction_at_alpha_0_1".into(), v["false_withhold_fraction"].clone());
        }
        v["alpha"] = json!(a);
        out.insert(name.to_string(), v);
    }
    out.insert("n_questions".into(), json!(n));
    out.insert("n_calibration".into(), json!(cal.len()));
    out.insert("n_test".into(), json!(test.len()));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run_withhold(7, 600), run_withhold(7, 600));
    }

    #[test]
    fn fixture_thirds() {
        let f = build_fixture(7, 300);
        for c in 0..3 {
            assert_eq!(f.iter().filter(|q| q.cond == c).count(), 100);
        }
    }

    #[test]
    fn calibrate_infeasible_all_wrong() {
        let cal: Vec<Q> = (0..20).map(|i| Q { cond: 0, correct: false, score: i as f64 }).collect();
        assert!(calibrate(&cal, 0.1).is_none());
    }

    #[test]
    fn calibrate_feasible_all_right() {
        let cal: Vec<Q> = (0..50).map(|i| Q { cond: 0, correct: true, score: i as f64 }).collect();
        assert_eq!(calibrate(&cal, 0.1), Some(0.0));
    }
}

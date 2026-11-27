//! Model-free shared-wrong simulation: a fixed-threshold support verifier that
//! passes misleading passages when parametric memory agrees with the passage,
//! reporting the false-withhold rate and where accepted wrong answers come from.

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde_json::{json, Value};

const CONDITIONS: [&str; 3] = ["sufficient", "missing", "misleading"];
const P_CORRECT: [f64; 3] = [0.9, 0.5, 0.2];
const P_PASS_HIGH: f64 = 0.95;
const P_PASS_LOW: f64 = 0.10;
const P_MEMORY_AGREE: f64 = 0.5;

#[derive(Clone, Copy)]
pub struct Q {
    pub cond: usize,
    pub agree: bool,
    pub correct: bool,
    pub accepted: bool,
}

pub fn build_fixture(seed: u64, n: usize) -> Vec<Q> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    (0..n)
        .map(|i| {
            let cond = i % 3;
            let agree = cond == 2 && rng.gen::<f64>() < P_MEMORY_AGREE;
            let correct = rng.gen::<f64>() < P_CORRECT[cond];
            let p_pass = if cond == 0 || agree { P_PASS_HIGH } else { P_PASS_LOW };
            let accepted = rng.gen::<f64>() < p_pass;
            Q { cond, agree, correct, accepted }
        })
        .collect()
}

fn frac(num: usize, den: usize) -> Value {
    if den == 0 { Value::Null } else { json!(num as f64 / den as f64) }
}

pub fn run_sharedwrong(seed: u64, n: usize) -> Value {
    let qs = build_fixture(seed, n);
    let withheld: Vec<&Q> = qs.iter().filter(|q| !q.accepted).collect();
    let fw = withheld.iter().filter(|q| q.correct).count();
    let acc_wrong: Vec<&Q> = qs.iter().filter(|q| q.accepted && !q.correct).collect();
    let n_acc = qs.iter().filter(|q| q.accepted).count();
    let agree_wrong = acc_wrong.iter().filter(|q| q.agree).count();
    let mut per = serde_json::Map::new();
    for (c, name) in CONDITIONS.iter().enumerate() {
        let w: Vec<&&Q> = withheld.iter().filter(|q| q.cond == c).collect();
        let wc = w.iter().filter(|q| q.correct).count();
        let aw = acc_wrong.iter().filter(|q| q.cond == c).count();
        per.insert(
            name.to_string(),
            json!({
                "n_withheld": w.len(),
                "n_false_withhold": wc,
                "n_accepted_wrong": aw,
            }),
        );
    }
    json!({
        "verifier": {
            "false_withhold_rate": frac(fw, withheld.len()),
            "n_withheld": withheld.len(),
            "n_false_withhold": fw,
            "n_accepted": n_acc,
            "n_accepted_wrong": acc_wrong.len(),
            "accepted_wrong_rate": frac(acc_wrong.len(), n_acc),
            "n_accepted_wrong_agree_misleading": agree_wrong,
            "agree_share_of_accepted_wrong": frac(agree_wrong, acc_wrong.len()),
            "per_condition": per,
            "n_questions": n,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run_sharedwrong(7, 600), run_sharedwrong(7, 600));
    }

    #[test]
    fn fixture_thirds() {
        let f = build_fixture(7, 300);
        for c in 0..3 {
            assert_eq!(f.iter().filter(|q| q.cond == c).count(), 100);
        }
        assert!(f.iter().all(|q| !q.agree || q.cond == 2));
    }

    #[test]
    fn counts_consistent() {
        let v = run_sharedwrong(7, 3000)["verifier"].clone();
        assert_eq!(
            v["n_withheld"].as_u64().unwrap() + v["n_accepted"].as_u64().unwrap(),
            3000
        );
        let s = v["agree_share_of_accepted_wrong"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&s));
    }
}

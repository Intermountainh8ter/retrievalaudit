//! Model-free abstention-layer simulation: a prompt-level layer lets a stated
//! share of misleading questions through to the shared-wrong verifier and
//! withholds the rest before verification.

use crate::sharedwrong::{build_fixture, Q};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde_json::{json, Value};

const STATED_ANSWER_RATE: f64 = 0.416;
const MISLEADING: usize = 2;

struct Stats {
    n_accepted: usize,
    n_accepted_wrong: usize,
    n_withheld: usize,
    n_false_withhold: usize,
}

impl Stats {
    fn to_json(&self) -> Value {
        json!({
            "n_accepted": self.n_accepted,
            "n_accepted_wrong": self.n_accepted_wrong,
            "n_withheld": self.n_withheld,
            "n_false_withhold": self.n_false_withhold,
            "accepted_wrong_rate": (self.n_accepted > 0).then(|| self.n_accepted_wrong as f64 / self.n_accepted as f64),
            "false_withhold_fraction": (self.n_withheld > 0).then(|| self.n_false_withhold as f64 / self.n_withheld as f64),
        })
    }
}

fn apply(qs: &[Q], draws: &[f64], answer_rate: f64) -> Stats {
    let mut s = Stats { n_accepted: 0, n_accepted_wrong: 0, n_withheld: 0, n_false_withhold: 0 };
    for (q, &u) in qs.iter().zip(draws) {
        let layer_abstains = q.cond == MISLEADING && u >= answer_rate;
        if layer_abstains || !q.accepted {
            s.n_withheld += 1;
            s.n_false_withhold += q.correct as usize;
        } else {
            s.n_accepted += 1;
            s.n_accepted_wrong += (!q.correct) as usize;
        }
    }
    s
}

pub fn run_abstain(seed: u64, n: usize) -> Value {
    let qs = build_fixture(seed, n);
    let mut rng = Pcg64Mcg::seed_from_u64(seed ^ 0xAB57_A1A1);
    let draws: Vec<f64> = (0..n).map(|_| rng.gen::<f64>()).collect();
    let alone = apply(&qs, &draws, 1.0);
    let layered = apply(&qs, &draws, STATED_ANSWER_RATE);
    let rate = |s: &Stats| s.n_accepted_wrong as f64 / s.n_accepted.max(1) as f64;
    let fwf = |s: &Stats| s.n_false_withhold as f64 / s.n_withheld.max(1) as f64;
    let a = layered.to_json();
    json!({
        "abstain": {
            "stated_misleading_answer_rate": STATED_ANSWER_RATE,
            "verifier_alone": alone.to_json(),
            "accepted_wrong_rate": a["accepted_wrong_rate"],
            "false_withhold_fraction": a["false_withhold_fraction"],
            "layered": a,
            "accepted_wrong_rate_change": rate(&layered) - rate(&alone),
            "false_withhold_fraction_change": fwf(&layered) - fwf(&alone),
            "n_questions": n,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run_abstain(7, 600), run_abstain(7, 600));
    }

    #[test]
    fn alone_matches_sharedwrong() {
        let a = run_abstain(7, 3000)["abstain"]["verifier_alone"].clone();
        let v = crate::sharedwrong::run_sharedwrong(7, 3000)["verifier"].clone();
        assert_eq!(a["n_accepted"], v["n_accepted"]);
        assert_eq!(a["n_false_withhold"], v["n_false_withhold"]);
    }

    #[test]
    fn layer_accepts_no_more() {
        let v = run_abstain(7, 3000)["abstain"].clone();
        assert!(v["layered"]["n_accepted"].as_u64() <= v["verifier_alone"]["n_accepted"].as_u64());
        for k in ["verifier_alone", "layered"] {
            assert_eq!(
                v[k]["n_accepted"].as_u64().unwrap() + v[k]["n_withheld"].as_u64().unwrap(),
                3000
            );
        }
    }
}

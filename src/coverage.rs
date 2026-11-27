//! Model-free coverage-cost simulation: adds a memory-conflict check of stated
//! strength to the shared-wrong verifier and reports how much the false-withhold
//! fraction rises when the accepted-wrong rate is cut to half.

use crate::sharedwrong::{build_fixture, Q};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde_json::{json, Value};

const FALSE_FIRE: f64 = 0.2;
const STATED_CATCH: f64 = 0.8;
const GRID: usize = 20;

struct Stats {
    n_accepted: usize,
    n_accepted_wrong: usize,
    n_withheld: usize,
    n_false_withhold: usize,
}

impl Stats {
    fn accepted_wrong_rate(&self) -> Option<f64> {
        (self.n_accepted > 0).then(|| self.n_accepted_wrong as f64 / self.n_accepted as f64)
    }
    fn false_withhold_fraction(&self) -> Option<f64> {
        (self.n_withheld > 0).then(|| self.n_false_withhold as f64 / self.n_withheld as f64)
    }
    fn to_json(&self) -> Value {
        json!({
            "n_accepted": self.n_accepted,
            "n_accepted_wrong": self.n_accepted_wrong,
            "n_withheld": self.n_withheld,
            "n_false_withhold": self.n_false_withhold,
            "accepted_wrong_rate": self.accepted_wrong_rate(),
            "false_withhold_fraction": self.false_withhold_fraction(),
        })
    }
}

/// The check looks only at answers the verifier accepted: it fires with
/// probability `catch` on memory-agreeing misleading answers and with
/// `FALSE_FIRE` on correct answers; other wrong answers are not seen by it.
fn apply(qs: &[Q], draws: &[f64], catch: f64, false_fire: f64) -> Stats {
    let mut s = Stats { n_accepted: 0, n_accepted_wrong: 0, n_withheld: 0, n_false_withhold: 0 };
    for (q, &u) in qs.iter().zip(draws) {
        let p_fire = if !q.accepted {
            1.0
        } else if q.agree {
            catch
        } else if q.correct {
            false_fire
        } else {
            0.0
        };
        if u < p_fire {
            s.n_withheld += 1;
            s.n_false_withhold += q.correct as usize;
        } else {
            s.n_accepted += 1;
            s.n_accepted_wrong += (!q.correct) as usize;
        }
    }
    s
}

pub fn run_coverage(seed: u64, n: usize) -> Value {
    let qs = build_fixture(seed, n);
    let mut rng = Pcg64Mcg::seed_from_u64(seed ^ 0xC0FE_C0FE);
    let draws: Vec<f64> = (0..n).map(|_| rng.gen::<f64>()).collect();
    // Unchecked verifier: nothing fires, so only originally withheld answers are withheld.
    let base = apply(&qs, &draws, 0.0, 0.0);
    let base_rate = base.accepted_wrong_rate();
    let base_fw = base.false_withhold_fraction();

    let mut grid = Vec::new();
    let mut half_at: Option<usize> = None;
    for i in 0..=GRID {
        let c = i as f64 / GRID as f64;
        let st = apply(&qs, &draws, c, FALSE_FIRE);
        let halved = match (st.accepted_wrong_rate(), base_rate) {
            (Some(r), Some(b)) => r <= b / 2.0,
            _ => false,
        };
        if halved && half_at.is_none() {
            half_at = Some(i);
        }
        let mut row = st.to_json();
        row["catch_rate"] = json!(c);
        row["halved"] = json!(halved);
        grid.push(row);
    }
    let feasible = half_at.is_some();
    let idx = half_at.unwrap_or(GRID);
    let at_half = apply(&qs, &draws, idx as f64 / GRID as f64, FALSE_FIRE);
    let increase = match (at_half.false_withhold_fraction(), base_fw) {
        (Some(a), Some(b)) => Some(a - b),
        _ => None,
    };
    let stated = apply(&qs, &draws, STATED_CATCH, FALSE_FIRE);
    let stated_halved = match (stated.accepted_wrong_rate(), base_rate) {
        (Some(r), Some(b)) => r <= b / 2.0,
        _ => false,
    };
    let always_answer = Stats {
        n_accepted: n,
        n_accepted_wrong: qs.iter().filter(|q| !q.correct).count(),
        n_withheld: 0,
        n_false_withhold: 0,
    };
    let always_withhold = Stats {
        n_accepted: 0,
        n_accepted_wrong: 0,
        n_withheld: n,
        n_false_withhold: qs.iter().filter(|q| q.correct).count(),
    };
    json!({
        "coverage": {
            "false_fire_rate": FALSE_FIRE,
            "stated_catch_rate": STATED_CATCH,
            "unchecked": base.to_json(),
            "stated_check": stated.to_json(),
            "stated_check_halves_accepted_wrong": stated_halved,
            "half_feasible_on_grid": feasible,
            "catch_rate_at_half": if feasible { json!(idx as f64 / GRID as f64) } else { Value::Null },
            "at_half_or_max_catch": at_half.to_json(),
            "false_withhold_increase_at_half_accepted_wrong": increase,
            "always_answer": always_answer.to_json(),
            "always_withhold": always_withhold.to_json(),
            "grid": grid,
            "n_questions": n,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run_coverage(7, 600), run_coverage(7, 600));
    }

    #[test]
    fn stated_check_accepts_fewer() {
        let v = run_coverage(7, 3000)["coverage"].clone();
        let g = v["grid"].as_array().unwrap();
        assert_eq!(g.len(), GRID + 1);
        let u = v["unchecked"]["n_accepted"].as_u64().unwrap();
        let s = v["stated_check"]["n_accepted"].as_u64().unwrap();
        assert!(s <= u);
    }

    #[test]
    fn unchecked_matches_sharedwrong() {
        let c = run_coverage(7, 3000)["coverage"]["unchecked"].clone();
        let v = crate::sharedwrong::run_sharedwrong(7, 3000)["verifier"].clone();
        assert_eq!(c["n_accepted"], v["n_accepted"]);
        assert_eq!(c["n_false_withhold"], v["n_false_withhold"]);
    }

    #[test]
    fn counts_partition() {
        let v = run_coverage(7, 3000)["coverage"].clone();
        for k in ["unchecked", "stated_check", "always_answer", "always_withhold"] {
            assert_eq!(
                v[k]["n_accepted"].as_u64().unwrap() + v[k]["n_withheld"].as_u64().unwrap(),
                3000
            );
        }
    }
}

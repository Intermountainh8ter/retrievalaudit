//! Model-free strictness sweep: an overlap-threshold verifier over synthetic
//! supported/unsupported claims; splits the rise in measured unsupported rate
//! into true detections and false rejections of supported claims.

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use serde_json::{json, Value};

const P_SUPPORTED: f64 = 0.7;
const SUPPORTED: (f64, f64) = (0.85, 0.12);
const UNSUPPORTED: (f64, f64) = (0.45, 0.20);
const THRESHOLDS: [f64; 10] = [0.50, 0.55, 0.60, 0.65, 0.70, 0.75, 0.80, 0.85, 0.90, 0.95];

fn normal(rng: &mut Pcg64Mcg, mean: f64, sd: f64) -> f64 {
    let u1: f64 = 1.0 - rng.gen::<f64>();
    let u2: f64 = rng.gen::<f64>();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    (mean + sd * z).clamp(0.0, 1.0)
}

/// (supported, score) per claim.
pub fn build_fixture(seed: u64, n: usize) -> Vec<(bool, f64)> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    (0..n)
        .map(|_| {
            let sup = rng.gen::<f64>() < P_SUPPORTED;
            let (m, s) = if sup { SUPPORTED } else { UNSUPPORTED };
            (sup, normal(&mut rng, m, s))
        })
        .collect()
}

pub fn run_strictness(seed: u64, n: usize) -> Value {
    let claims = build_fixture(seed, n);
    let nf = n as f64;
    let mut rows = Vec::new();
    let mut monotone = true;
    let mut prev = f64::NEG_INFINITY;
    let mut first = (0usize, 0usize);
    let mut last = (0usize, 0usize);
    for (i, &t) in THRESHOLDS.iter().enumerate() {
        let det = claims.iter().filter(|(s, x)| !*s && *x < t).count();
        let fr = claims.iter().filter(|(s, x)| *s && *x < t).count();
        let rate = (det + fr) as f64 / nf;
        if rate < prev {
            monotone = false;
        }
        prev = rate;
        if i == 0 {
            first = (det, fr);
        }
        last = (det, fr);
        rows.push(json!({
            "threshold": t,
            "measured_unsupported_rate": rate,
            "n_true_detections": det,
            "n_false_rejections": fr,
        }));
    }
    let rise_det = last.0 - first.0.min(last.0);
    let rise_fr = last.1 - first.1.min(last.1);
    let rise = rise_det + rise_fr;
    let share = if rise == 0 { Value::Null } else { json!(rise_fr as f64 / rise as f64) };
    json!({
        "strictness_sweep": {
            "grid": rows,
            "n_claims": n,
            "n_supported": claims.iter().filter(|(s, _)| *s).count(),
            "rise_in_measured_unsupported_rate": rise as f64 / nf,
            "rise_true_detections": rise_det,
            "rise_false_rejections": rise_fr,
            "false_rejection_share_of_rise": share,
            "monotone_increasing": monotone,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(run_strictness(7, 500), run_strictness(7, 500));
    }

    #[test]
    fn rates_monotone_and_bounded() {
        let v = run_strictness(7, 3000)["strictness_sweep"].clone();
        assert_eq!(v["monotone_increasing"], json!(true));
        let s = v["false_rejection_share_of_rise"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&s));
    }

    #[test]
    fn scores_in_unit_interval() {
        assert!(build_fixture(3, 1000).iter().all(|(_, x)| (0.0..=1.0).contains(x)));
    }
}

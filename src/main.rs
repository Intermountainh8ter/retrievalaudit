//! retrievalaudit CLI.
//!
//! `canon <manifest>` — print canonical manifest SHA.
//! `audit --manifest m.json --retrievals r.jsonl --truths t.jsonl --generations g.jsonl`
//!   — compute precision/recall@k, faithfulness, with 95% bootstrap CIs.
//!   Exit 0 Scored, 1 EmptyRetrieval, 2 Unverified.

use clap::{Parser, Subcommand};
use retrievalaudit::{
    audit, is_degenerate, manifest_sha256_hex, GenerationResult, GroundTruth, RagManifest,
    RetrievalResult, Verdict,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "retrievalaudit", version, about = "Pre-registered RAG reproducibility audit")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Canonical SHA-256 of a manifest.
    Canon {
        manifest: PathBuf,
    },
    /// Audit a full fixture. Exit code 0/1/2 for Scored/EmptyRetrieval/Unverified.
    Audit {
        #[arg(long)]
        manifest: PathBuf,
        /// JSONL file: one RetrievalResult per line.
        #[arg(long)]
        retrievals: PathBuf,
        /// JSONL file: one GroundTruth per line, paired by query_id.
        #[arg(long)]
        truths: PathBuf,
        /// JSONL file: one GenerationResult per line.
        #[arg(long)]
        generations: PathBuf,
        #[arg(long, default_value_t = 5)]
        min_token_len: usize,
        #[arg(long, default_value_t = 2000)]
        boot: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Model-free lexical strictness ladder on a seeded synthetic fixture.
    Ladder {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 400)]
        n_claims: usize,
        #[arg(long, default_value_t = 2000)]
        boot: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// Model-free split-conformal withhold simulation on a seeded synthetic fixture.
    Withhold {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 3000)]
        n_questions: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// Print the version banner and exit.
    Version,
}

fn read_jsonl<T: serde::de::DeserializeOwned>(p: &PathBuf) -> anyhow::Result<Vec<T>> {
    let text = std::fs::read_to_string(p)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        out.push(serde_json::from_str(t).map_err(|e| anyhow::anyhow!("line {}: {e}", i + 1))?);
    }
    Ok(out)
}

fn utc_iso(secs: u64) -> String {
    let (days, rem) = (secs / 86400, secs % 86400);
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Canon { manifest } => {
            let m: RagManifest = serde_json::from_str(&std::fs::read_to_string(&manifest)?)?;
            if let Some(r) = is_degenerate(&m) {
                eprintln!("manifest is degenerate: {r}");
                std::process::exit(2);
            }
            println!("{}", manifest_sha256_hex(&m));
        }
        Cmd::Audit {
            manifest,
            retrievals,
            truths,
            generations,
            min_token_len,
            boot,
            seed,
        } => {
            let m: RagManifest = serde_json::from_str(&std::fs::read_to_string(&manifest)?)?;
            let retrievals: Vec<RetrievalResult> = read_jsonl(&retrievals)?;
            let truths: Vec<GroundTruth> = read_jsonl(&truths)?;
            let generations: Vec<GenerationResult> = read_jsonl(&generations)?;
            let r = audit(&m, &retrievals, &truths, &generations, min_token_len, boot, seed);
            println!("{}", serde_json::to_string_pretty(&r)?);
            match r.verdict {
                Verdict::Scored => {}
                Verdict::EmptyRetrieval => std::process::exit(1),
                Verdict::Unverified(_) => std::process::exit(2),
            }
        }
        Cmd::Ladder { seed, n_claims, boot, out } => {
            let t0 = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let ladder = retrievalaudit::ladder::run_ladder(seed, n_claims, boot);
            let doc = serde_json::json!({
                "ladder": ladder,
                "provenance": {
                    "started_at": utc_iso(started),
                    "seeds": [seed],
                    "command": format!("retrievalaudit ladder --seed {seed} --n-claims {n_claims} --boot {boot} --out {}", out.display()),
                    "hardware": "cpu",
                    "wall_s": t0.elapsed().as_secs_f64(),
                    "data_sources": [{"url": "synthetic: generated in-crate from seed, no external data", "license": "MIT"}],
                },
            });
            if let Some(d) = out.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::write(&out, serde_json::to_string_pretty(&doc)? + "\n")?;
        }
        Cmd::Withhold { seed, n_questions, out } => {
            let t0 = std::time::Instant::now();
            let started = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let withhold = retrievalaudit::withhold::run_withhold(seed, n_questions);
            let doc = serde_json::json!({
                "withhold": withhold,
                "provenance": {
                    "started_at": utc_iso(started),
                    "seeds": [seed],
                    "command": format!("retrievalaudit withhold --seed {seed} --n-questions {n_questions} --out {}", out.display()),
                    "hardware": "cpu",
                    "wall_s": t0.elapsed().as_secs_f64(),
                    "data_sources": [{"url": "synthetic: generated in-crate from seed, no external data", "license": "MIT"}],
                },
            });
            if let Some(d) = out.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::write(&out, serde_json::to_string_pretty(&doc)? + "\n")?;
        }
        Cmd::Version => println!("retrievalaudit {}", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}

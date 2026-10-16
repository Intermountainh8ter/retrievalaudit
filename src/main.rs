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
        Cmd::Version => println!("retrievalaudit {}", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}

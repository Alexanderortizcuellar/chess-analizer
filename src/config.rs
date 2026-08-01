use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about = "Chess Analyzer - high-performance chess analysis toolkit")]
pub struct CliArgs {
    #[arg(short, long, help = "Path to the configuration JSON file")]
    pub config: Option<String>,

    #[arg(short, long, help = "Path to Stockfish engine executable")]
    pub engine: Option<String>,

    #[arg(short, long, help = "Engine search depth")]
    pub depth: Option<u32>,

    #[arg(short, long, help = "Number of concurrent processes (integer or 'auto')")]
    pub processes: Option<String>,

    #[arg(short, long, help = "Threads per Stockfish process")]
    pub threads: Option<u32>,

    #[arg(short = 'm', long, help = "Hash size (MB) per Stockfish process")]
    pub hash: Option<u32>,

    #[arg(long, help = "Maximum search time (seconds) per position (default: 20)")]
    pub movetime: Option<u32>,

    #[arg(long, help = "Comma-separated output formats (e.g., 'pgn,json')")]
    pub format: Option<String>,

    #[arg(short, long, help = "Directory to save output files")]
    pub output_dir: Option<String>,

    #[arg(required = true, help = "Input PGN file(s)")]
    pub input_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EngineConfigJson {
    pub path: Option<String>,
    pub depth: Option<u32>,
    pub processes: Option<String>,
    pub threads_per_process: Option<u32>,
    pub hash_mb: Option<u32>,
    pub movetime_sec: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutputConfigJson {
    pub format: Option<Vec<String>>,
    pub output_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigJson {
    pub engine: Option<EngineConfigJson>,
    pub output: Option<OutputConfigJson>,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub engine_path: String,
    pub depth: u32,
    pub processes: String,
    pub threads_per_process: Option<u32>,
    pub hash_mb: Option<u32>,
    pub movetime_sec: u32,
    pub formats: Vec<String>,
    pub output_dir: String,
    pub input_files: Vec<String>,
}

impl ResolvedConfig {
    pub fn resolve() -> Result<Self, String> {
        let cli = CliArgs::parse();
        
        // 1. Try loading config JSON if specified
        let mut config_json = ConfigJson { engine: None, output: None };
        if let Some(config_path) = &cli.config {
            let path = Path::new(config_path);
            if !path.exists() {
                return Err(format!("Configuration file not found: {}", config_path));
            }
            let mut file = File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).map_err(|e| format!("Failed to read config file: {}", e))?;
            config_json = serde_json::from_str(&contents).map_err(|e| format!("Failed to parse config JSON: {}", e))?;
        } else {
            // Check if default config.json exists in the current directory
            let default_path = Path::new("config.json");
            if default_path.exists() {
                if let Ok(mut file) = File::open(default_path) {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        if let Ok(parsed) = serde_json::from_str::<ConfigJson>(&contents) {
                            config_json = parsed;
                        }
                    }
                }
            }
        }

        // Helper to check standard Stockfish executable paths
        let find_stockfish = || -> String {
            // First choice: check the path we found on the user's system
            let known_path = "C:\\Users\\ASUS\\chess\\engines\\stockfish.exe";
            if Path::new(known_path).exists() {
                return known_path.to_string();
            }
            // Fallback to "stockfish" and let OS resolve it in PATH
            "stockfish".to_string()
        };

        // 2. Resolve parameters with overrides
        let engine_path = cli.engine
            .or_else(|| config_json.engine.as_ref().and_then(|e| e.path.clone()))
            .unwrap_or_else(find_stockfish);

        let depth = cli.depth
            .or_else(|| config_json.engine.as_ref().and_then(|e| e.depth))
            .unwrap_or(15); // Default to depth 15 (fast for testing/general usage)

        let processes = cli.processes
            .or_else(|| config_json.engine.as_ref().and_then(|e| e.processes.clone()))
            .unwrap_or_else(|| "auto".to_string());

        let threads_per_process = cli.threads
            .or_else(|| config_json.engine.as_ref().and_then(|e| e.threads_per_process));

        let hash_mb = cli.hash
            .or_else(|| config_json.engine.as_ref().and_then(|e| e.hash_mb));

        let movetime_sec = cli.movetime
            .or_else(|| config_json.engine.as_ref().and_then(|e| e.movetime_sec))
            .unwrap_or(20); // Default to 20 seconds limit

        let formats = if let Some(fmt_str) = cli.format {
            fmt_str.split(',').map(|s| s.trim().to_lowercase()).collect()
        } else if let Some(fmts) = config_json.output.as_ref().and_then(|o| o.format.clone()) {
            fmts.iter().map(|s| s.trim().to_lowercase()).collect()
        } else {
            vec!["pgn".to_string(), "json".to_string()]
        };

        let output_dir = cli.output_dir
            .or_else(|| config_json.output.as_ref().and_then(|o| o.output_dir.clone()))
            .unwrap_or_else(|| ".".to_string());

        Ok(ResolvedConfig {
            engine_path,
            depth,
            processes,
            threads_per_process,
            hash_mb,
            movetime_sec,
            formats,
            output_dir,
            input_files: cli.input_files,
        })
    }
}

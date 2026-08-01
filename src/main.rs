use std::fs;
use std::path::Path;
use chess_analizer::config::ResolvedConfig;
use chess_analizer::cache::SharedPositionCache;
use chess_analizer::scheduler::{SchedulerConfig, analyze_games};
use chess_analizer::annotations::{ClassificationConfig, classify_all_moves};
use chess_analizer::tactics::detect_all_tactics;
use chess_analizer::exporter::{export_to_pgn, export_to_json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Resolve configuration
    let config = match ResolvedConfig::resolve() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    println!("Starting Chess Analyzer...");
    println!("Engine path: {}", config.engine_path);
    println!("Depth: {}", config.depth);
    println!("Output directory: {}", config.output_dir);
    println!("Input files: {:?}", config.input_files);

    // Ensure output directory exists
    let out_dir = Path::new(&config.output_dir);
    if !out_dir.exists() {
        fs::create_dir_all(out_dir)?;
    }

    // 2. Read and parse games from all input PGN files
    let mut all_games = Vec::new();
    for pgn_path in &config.input_files {
        println!("Reading input PGN: {}", pgn_path);
        let contents = fs::read_to_string(pgn_path)?;
        let parsed = chess_analizer::pgn::parse_pgn(&contents);
        println!("Parsed {} game(s) from {}", parsed.len(), pgn_path);
        all_games.extend(parsed);
    }

    if all_games.is_empty() {
        println!("No games found to analyze. Exiting.");
        return Ok(());
    }

    // 3. Initialize SharedPositionCache
    let cache = SharedPositionCache::new();

    // 4. Run scheduler to perform Stockfish analysis
    let scheduler_config = SchedulerConfig {
        engine_path: config.engine_path.clone(),
        depth: config.depth,
        num_processes: config.processes.clone(),
        threads_per_process: config.threads_per_process,
        hash_mb_per_process: config.hash_mb,
    };

    println!("Analyzing games using Stockfish...");
    analyze_games(&mut all_games, &scheduler_config, &cache).await?;

    // 5. Run move classification
    println!("Classifying moves...");
    let class_config = ClassificationConfig::default();
    classify_all_moves(&mut all_games, &class_config);

    // 6. Run tactical detection
    println!("Detecting tactical opportunities...");
    detect_all_tactics(&mut all_games);

    // 7. Export results
    for format in &config.formats {
        match format.as_str() {
            "pgn" => {
                let pgn_out = export_to_pgn(&all_games);
                let output_path = out_dir.join("annotated_games.pgn");
                fs::write(&output_path, pgn_out)?;
                println!("Exported annotated PGN to: {:?}", output_path);
            }
            "json" => {
                let json_out = export_to_json(&all_games)?;
                let output_path = out_dir.join("analyzed_games.json");
                fs::write(&output_path, json_out)?;
                println!("Exported structured JSON to: {:?}", output_path);
            }
            other => {
                println!("Warning: Unknown output format '{}' skipped.", other);
            }
        }
    }

    println!("Chess Analyzer finished successfully!");
    Ok(())
}

use std::env;
use std::fs;
use std::collections::HashSet;
use chess_analizer::pgn::parse_pgn;
use chess_analizer::cache::normalize_fen;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --release --bin count_unique -- <path_to_pgn>");
        return;
    }
    
    let path = &args[1];
    println!("Reading PGN from: {} ...", path);
    
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return;
        }
    };
    
    let games = parse_pgn(&content);
    println!("Parsed {} games.", games.len());
    
    if games.is_empty() {
        println!("No games found in the file.");
        return;
    }
    
    let mut unique_fens = HashSet::new();
    let mut total_positions = 0;
    
    for game in &games {
        for (idx, mv) in game.moves.iter().enumerate() {
            total_positions += 1;
            let normalized_before = normalize_fen(&mv.fen_before);
            unique_fens.insert(normalized_before);
            
            if idx == game.moves.len() - 1 {
                total_positions += 1;
                let normalized_after = normalize_fen(&mv.fen_after);
                unique_fens.insert(normalized_after);
            }
        }
    }
    
    println!("\n=== Metrics for Chess Games ===");
    println!("Total positions across games: {}", total_positions);
    println!("Unique positions (deduplicated): {}", unique_fens.len());
    
    // Calculate estimates
    let uniq_count = unique_fens.len();
    println!("\n=== Estimated Run Time on 64-vCPU VM (using 60 parallel threads) ===");
    println!("Formula: (unique_positions * average_search_time_seconds) / 60 / 3600");
    
    let time_20s = (uniq_count as f64 * 20.0) / 60.0 / 3600.0;
    let time_30s = (uniq_count as f64 * 30.0) / 60.0 / 3600.0;
    let time_45s = (uniq_count as f64 * 45.0) / 60.0 / 3600.0;
    
    println!("- At 20 seconds average per position: {:.2} hours", time_20s);
    println!("- At 30 seconds average per position: {:.2} hours", time_30s);
    println!("- At 45 seconds average per position: {:.2} hours", time_45s);
}

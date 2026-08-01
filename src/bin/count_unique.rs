use std::env;
use std::fs;
use std::collections::HashSet;
use chess_analizer::pgn::parse_pgn;
use chess_analizer::cache::normalize_fen;
use chess_analizer::model::Game;
use chess_analizer::exporter::export_to_pgn;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --release --bin count_unique -- <path_to_pgn> [output_filtered_pgn]");
        return;
    }
    
    let path = &args[1];
    let output_path = args.get(2);
    
    println!("Reading PGN from: {} ...", path);
    
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return;
        }
    };
    
    let games = parse_pgn(&content);
    println!("Parsed {} total games from input file.", games.len());
    
    // Filter games from 2024 and later
    let filtered_games: Vec<Game> = games.into_iter()
        .filter(|game| {
            if let Some(year) = get_game_year(game) {
                year >= 2024
            } else {
                false
            }
        })
        .collect();
        
    println!("Filtered down to {} games (from 2024 and later).", filtered_games.len());
    
    if filtered_games.is_empty() {
        println!("No games found matching 2024 or later.");
        return;
    }
    
    // Write filtered games to output path if specified
    if let Some(out_p) = output_path {
        println!("Writing filtered PGN to: {} ...", out_p);
        let pgn_text = export_to_pgn(&filtered_games);
        if let Err(e) = fs::write(out_p, pgn_text) {
            eprintln!("Error writing filtered PGN file: {}", e);
        } else {
            println!("Successfully wrote filtered PGN file.");
        }
    }
    
    let mut unique_fens = HashSet::new();
    let mut total_positions = 0;
    
    for game in &filtered_games {
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
    
    println!("\n=== Metrics for 2024 and Later Games ===");
    println!("Total positions across filtered games: {}", total_positions);
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

fn get_game_year(game: &Game) -> Option<i32> {
    let date_str = game.get_header("Date")
        .or_else(|| game.get_header("date"))?;
        
    if date_str.len() >= 4 {
        date_str[0..4].parse::<i32>().ok()
    } else {
        None
    }
}

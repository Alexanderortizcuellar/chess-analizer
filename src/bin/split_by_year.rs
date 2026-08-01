use std::fs;
use std::path::Path;
use std::collections::HashMap;
use chess_analizer::pgn::parse_pgn;
use chess_analizer::exporter::export_to_pgn;
use chess_analizer::model::Game;

fn main() {
    let pgn_path = "C:\\Users\\ASUS\\programming\\qt_programs\\chess\\downloader\\alex.pgn";
    println!("Reading PGN from: {} ...", pgn_path);
    
    let content = match fs::read_to_string(pgn_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return;
        }
    };
    
    let games = parse_pgn(&content);
    println!("Parsed {} games.", games.len());
    
    // Group games by year
    let mut games_by_year: HashMap<String, Vec<Game>> = HashMap::new();
    
    for game in games {
        let year_str = if let Some(year) = get_game_year(&game) {
            year.to_string()
        } else {
            "unknown".to_string()
        };
        
        games_by_year.entry(year_str).or_default().push(game);
    }
    
    // Create games directory inside the repo
    let games_dir = Path::new("games");
    if !games_dir.exists() {
        let _ = fs::create_dir(games_dir);
    }
    
    for (year, games_subset) in games_by_year {
        let file_name = format!("games/alex_{}.pgn", year);
        println!("Writing {} games to {} ...", games_subset.len(), file_name);
        
        let pgn_text = export_to_pgn(&games_subset);
        if let Err(e) = fs::write(&file_name, pgn_text) {
            eprintln!("Error writing file {}: {}", file_name, e);
        }
    }
    
    println!("Splitting completed!");
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

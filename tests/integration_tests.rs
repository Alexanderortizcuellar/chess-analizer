use chess_analizer::pgn::parse_pgn;

#[test]
fn test_pgn_parsing() {
    let pgn_content = r#"[Event "Scholar's Mate Example"]
[Site "Local Test"]
[Date "2026.07.31"]
[Round "1"]
[White "Scholar"]
[Black "Target"]
[Result "1-0"]

1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# 1-0
"#;

    let games = parse_pgn(pgn_content);
    assert_eq!(games.len(), 1);
    
    let game = &games[0];
    assert_eq!(game.get_header("White"), Some("Scholar"));
    assert_eq!(game.get_header("Black"), Some("Target"));
    assert_eq!(game.get_header("Result"), Some("1-0"));
    
    // Scholar's Mate is:
    // 1. e4 (ply 0)
    // 1... e5 (ply 1)
    // 2. Qh5 (ply 2)
    // 2... Nc6 (ply 3)
    // 3. Bc4 (ply 4)
    // 3... Nf6 (ply 5)
    // 4. Qxf7# (ply 6)
    assert_eq!(game.moves.len(), 7);
    
    assert_eq!(game.moves[0].san, "e4");
    assert_eq!(game.moves[0].uci, "e2e4");
    assert_eq!(game.moves[0].move_number, 1);
    assert_eq!(game.moves[0].is_white, true);
    
    assert_eq!(game.moves[1].san, "e5");
    assert_eq!(game.moves[1].uci, "e7e5");
    assert_eq!(game.moves[1].move_number, 1);
    assert_eq!(game.moves[1].is_white, false);
    
    assert_eq!(game.moves[5].san, "Nf6");
    assert_eq!(game.moves[5].uci, "g8f6");
    
    assert_eq!(game.moves[6].san, "Qxf7#");
    assert_eq!(game.moves[6].uci, "h5f7");
}

#[test]
fn test_miss_classification() {
    use chess_analizer::model::{Game, AnalyzedMove, Evaluation, MoveClassification};
    use chess_analizer::annotations::{ClassificationConfig, classify_game_moves};

    let mut game = Game {
        headers: Vec::new(),
        moves: vec![
            AnalyzedMove {
                move_number: 1,
                ply: 0,
                is_white: true,
                san: "Nf3".to_string(),
                uci: "g1f3".to_string(),
                fen_before: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -".to_string(),
                fen_after: "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq -".to_string(),
                evaluation: Some(Evaluation::Centipawns(300)), // +3.0 (was winning)
                classification: None,
                tactical_findings: Vec::new(),
                engine_metadata: None,
                comment: None,
            }
        ],
        final_evaluation: Some(Evaluation::Centipawns(50)), // +0.5 (dropped to equal-ish, not losing)
    };
    
    let config = ClassificationConfig::default();
    classify_game_moves(&mut game, &config);
    
    assert_eq!(game.moves[0].classification, Some(MoveClassification::Miss));
}

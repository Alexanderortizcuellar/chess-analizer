use std::ops::ControlFlow;
use std::str;
use pgn_reader::{Visitor, Reader, SanPlus, RawTag, RawComment, Skip, Outcome, Nag};
use shakmaty::{Chess, Position, CastlingMode, EnPassantMode};
use shakmaty::fen::{Fen, Epd};
use shakmaty::san::San;
use crate::model::{Game, AnalyzedMove};

pub struct GameVisitor {
    pub current_game: Option<Game>,
}

impl GameVisitor {
    pub fn new() -> Self {
        Self { current_game: None }
    }
}

// Associated types for pgn-reader Visitor
impl Visitor for GameVisitor {
    type Tags = Vec<(String, String)>;
    type Movetext = (Vec<(String, String)>, Vec<AnalyzedMove>, Chess);
    type Output = Option<Game>;

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(Vec::new())
    }

    fn tag(&mut self, tags: &mut Self::Tags, key: &[u8], value: RawTag<'_>) -> ControlFlow<Self::Output> {
        let k = str::from_utf8(key).unwrap_or("").to_string();
        let v = str::from_utf8(value.as_bytes()).unwrap_or("").to_string();
        tags.push((k, v));
        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        // Look for FEN tag to set up initial board state, default is starting position
        let mut pos = Chess::default();
        if let Some((_, fen_val)) = tags.iter().find(|(k, _)| k.eq_ignore_ascii_case("fen")) {
            if let Ok(fen) = fen_val.parse::<Fen>() {
                if let Ok(p) = fen.into_position(CastlingMode::Standard) {
                    pos = p;
                }
            }
        }
        ControlFlow::Continue((tags, Vec::new(), pos))
    }

    fn san(&mut self, movetext: &mut Self::Movetext, san_plus: SanPlus) -> ControlFlow<Self::Output> {
        let (_tags, moves, pos) = movetext;
        
        let san_str = san_plus.to_string();
        let fen_before = Epd::from_position(pos as &Chess, EnPassantMode::Legal).to_string();
        
        // Parse SAN in shakmaty
        if let Ok(san) = san_str.parse::<San>() {
            if let Ok(mv) = san.to_move(pos as &Chess) {
                let uci = mv.to_uci(shakmaty::CastlingMode::Standard).to_string();
                
                // Play move
                if let Ok(new_pos) = pos.clone().play(mv) {
                    *pos = new_pos;
                    let fen_after = Epd::from_position(pos as &Chess, EnPassantMode::Legal).to_string();
                    
                    let ply = moves.len() as u32;
                    let move_number = 1 + ply / 2;
                    let is_white = ply % 2 == 0;
                    
                    moves.push(AnalyzedMove {
                        move_number,
                        ply,
                        is_white,
                        san: san_str,
                        uci,
                        fen_before,
                        fen_after,
                        evaluation: None,
                        classification: None,
                        tactical_findings: Vec::new(),
                        engine_metadata: None,
                        comment: None,
                    });
                }
            }
        }
        ControlFlow::Continue(())
    }

    fn comment(&mut self, movetext: &mut Self::Movetext, comment: RawComment<'_>) -> ControlFlow<Self::Output> {
        let (_tags, moves, _pos) = movetext;
        if let Some(last_move) = moves.last_mut() {
            let comm_str = str::from_utf8(comment.as_bytes()).unwrap_or("").trim().to_string();
            if !comm_str.is_empty() {
                // If there's already a comment, append it or update it
                if let Some(existing) = &mut last_move.comment {
                    existing.push_str(" ");
                    existing.push_str(&comm_str);
                } else {
                    last_move.comment = Some(comm_str);
                }
            }
        }
        ControlFlow::Continue(())
    }

    fn nag(&mut self, _movetext: &mut Self::Movetext, _nag: Nag) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn begin_variation(&mut self, _movetext: &mut Self::Movetext) -> ControlFlow<Self::Output, Skip> {
        // Skip sub-variations for simplicity
        ControlFlow::Continue(Skip(true))
    }

    fn end_variation(&mut self, _movetext: &mut Self::Movetext) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn outcome(&mut self, _movetext: &mut Self::Movetext, _outcome: Outcome) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn end_game(&mut self, movetext: Self::Movetext) -> Self::Output {
        let (tags, moves, _pos) = movetext;
        let game = Game {
            headers: tags,
            moves,
            final_evaluation: None,
        };
        self.current_game = Some(game.clone());
        Some(game)
    }
}

pub fn parse_pgn(pgn_content: &str) -> Vec<Game> {
    let mut reader = Reader::new(pgn_content.as_bytes());
    let mut visitor = GameVisitor::new();
    let mut games = Vec::new();
    
    while let Ok(Some(maybe_game)) = reader.read_game(&mut visitor) {
        if let Some(game) = maybe_game {
            games.push(game);
        }
    }
    games
}

use std::fmt::Write;
use shakmaty::fen::Fen;
use shakmaty::{Chess, Position};
use shakmaty::san::San;
use crate::model::Game;

pub fn export_to_json(games: &[Game]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(games)
}

pub fn export_to_pgn(games: &[Game]) -> String {
    let mut pgn = String::new();
    
    for game in games {
        // Headers
        for (k, v) in &game.headers {
            let _ = writeln!(pgn, "[{} \"{}\"]", k, v);
        }
        let _ = writeln!(pgn); // empty line after headers
        
        // Moves
        let mut move_line = String::new();
        let moves_len = game.moves.len();
        
        for (idx, mv) in game.moves.iter().enumerate() {
            let mut move_str = String::new();
            
            // Output move number
            if mv.is_white {
                let _ = write!(move_str, "{}. {}", mv.move_number, mv.san);
            } else {
                // If Black's move, we only print number if the previous move had a comment
                let prev_had_comment = if idx > 0 {
                    game.moves[idx - 1].comment.is_some() || 
                    game.moves[idx - 1].evaluation.is_some() || 
                    game.moves[idx - 1].engine_metadata.is_some()
                } else {
                    false
                };
                
                if prev_had_comment {
                    let _ = write!(move_str, "{}... {}", mv.move_number, mv.san);
                } else {
                    let _ = write!(move_str, "{}", mv.san);
                }
            }
            
            // Build comment
            let mut comment_parts = Vec::new();
            
            if let Some(eval) = &mv.evaluation {
                comment_parts.push(format!("[%eval {}]", eval));
            }
            
            let mut pv_variation = None;
            
            if let Some(meta) = &mv.engine_metadata {
                let mut best_move_san = meta.best_move.clone();
                if let Ok(fen) = mv.fen_before.parse::<Fen>() {
                    if let Ok(pos_before) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                        if let Ok(best_mv) = parse_uci_move(&pos_before, &meta.best_move) {
                            best_move_san = San::from_move(&pos_before, best_mv).to_string();
                        }
                        
                        // Check if we have a forced mate or forced win to format the full variation
                        let is_forced_mate_or_win = if let Some(eval) = &mv.evaluation {
                            match eval {
                                crate::model::Evaluation::Mate(_) => true,
                                crate::model::Evaluation::Centipawns(cp) => cp.abs() >= 300,
                            }
                        } else {
                            false
                        };
                        
                        if is_forced_mate_or_win && !meta.principal_variation.is_empty() {
                            pv_variation = format_pv_variation(
                                &pos_before,
                                &meta.principal_variation,
                                mv.is_white,
                                mv.move_number,
                            );
                        }
                    }
                }
                
                comment_parts.push(format!("[%bestmove {}]", best_move_san));
            }
            
            // Add classification & tactical findings inside [%analysis ...]
            let mut analysis_tokens = Vec::new();
            if let Some(cls) = &mv.classification {
                analysis_tokens.push(format!("classification={:?}", cls));
            }
            
            // Find centipawn loss for score_loss
            if let Some(eval_before) = &mv.evaluation {
                let eval_after = if idx + 1 < moves_len {
                    game.moves[idx + 1].evaluation.clone()
                } else {
                    game.final_evaluation.clone()
                };
                
                if let Some(eval_after) = eval_after {
                    let val_before = eval_before.absolute_value();
                    let val_after = eval_after.absolute_value();
                    let loss = if mv.is_white {
                        val_before - val_after
                    } else {
                        val_after - val_before
                    };
                    if loss > 0.0 {
                        analysis_tokens.push(format!("score_loss={:.2}", loss));
                    }
                }
            }
            
            if !mv.tactical_findings.is_empty() {
                let findings: Vec<String> = mv.tactical_findings.iter().map(|f| format!("{:?}", f)).collect();
                analysis_tokens.push(format!("missed={}", findings.join(",")));
            }
            
            if !analysis_tokens.is_empty() {
                comment_parts.push(format!("[%analysis {}]", analysis_tokens.join(" ")));
            }
            
            // Include original move comment
            if let Some(orig_comm) = &mv.comment {
                comment_parts.push(orig_comm.clone());
            }
            
            if !comment_parts.is_empty() {
                let _ = write!(move_str, " {{ {} }}", comment_parts.join(" "));
            }
            
            if let Some(var_str) = pv_variation {
                let _ = write!(move_str, " ({})", var_str);
            }
            
            // Append to line, managing simple wrapping
            if move_line.is_empty() {
                move_line = move_str;
            } else {
                if move_line.len() + move_str.len() + 1 > 80 {
                    let _ = writeln!(pgn, "{}", move_line);
                    move_line = move_str;
                } else {
                    move_line.push(' ');
                    move_line.push_str(&move_str);
                }
            }
        }
        
        // Output final line
        if !move_line.is_empty() {
            let _ = writeln!(pgn, "{}", move_line);
        }
        
        // Output game outcome if present in headers, or standard wildcard
        let outcome = game.get_header("result").unwrap_or("*");
        let _ = writeln!(pgn, "{}", outcome);
        let _ = writeln!(pgn); // blank line between games
    }
    
    pgn
}

// Format the principal variation (PV) as standard PGN notation
fn format_pv_variation(
    pos_before: &Chess,
    pv: &[String],
    is_white_to_move: bool,
    start_move_number: u32,
) -> Option<String> {
    if pv.is_empty() {
        return None;
    }
    
    let mut variation = String::new();
    let mut current_pos = pos_before.clone();
    let mut move_number = start_move_number;
    let mut is_white = is_white_to_move;
    
    for (i, uci) in pv.iter().enumerate() {
        if let Ok(mv) = parse_uci_move(&current_pos, uci) {
            let san = San::from_move(&current_pos, mv.clone()).to_string();
            
            // Format move number
            if is_white {
                if i == 0 {
                    let _ = write!(variation, "{}. {}", move_number, san);
                } else {
                    let _ = write!(variation, " {}. {}", move_number, san);
                }
            } else {
                if i == 0 {
                    let _ = write!(variation, "{}... {}", move_number, san);
                } else {
                    let _ = write!(variation, " {}", san);
                }
            }
            
            // Play the move to update position
            if let Ok(next_pos) = current_pos.play(mv) {
                current_pos = next_pos;
            } else {
                break;
            }
            
            // Toggle side to move
            if !is_white {
                move_number += 1;
            }
            is_white = !is_white;
        } else {
            break;
        }
    }
    
    if variation.is_empty() {
        None
    } else {
        Some(variation)
    }
}

// Helper to parse UCI move in pos context
fn parse_uci_move(pos: &Chess, uci: &str) -> Result<shakmaty::Move, String> {
    if uci.len() < 4 {
        return Err("Invalid UCI length".to_string());
    }
    let from = uci[0..2].parse::<shakmaty::Square>().map_err(|_| "Invalid from square")?;
    let to = uci[2..4].parse::<shakmaty::Square>().map_err(|_| "Invalid to square")?;
    let promotion = if uci.len() > 4 {
        let ch = uci.chars().nth(4).unwrap();
        match ch {
            'q' => Some(shakmaty::Role::Queen),
            'r' => Some(shakmaty::Role::Rook),
            'b' => Some(shakmaty::Role::Bishop),
            'n' => Some(shakmaty::Role::Knight),
            _ => None,
        }
    } else {
        None
    };
    for mv in pos.legal_moves() {
        if mv.from() == Some(from) && mv.to() == to {
            if let Some(promo) = promotion {
                if mv.promotion() == Some(promo) {
                    return Ok(mv);
                }
            } else if mv.promotion().is_none() {
                return Ok(mv);
            }
        }
    }
    Err(format!("No legal move matches UCI: {}", uci))
}

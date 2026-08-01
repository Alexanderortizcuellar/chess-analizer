use shakmaty::{Chess, Position};
use shakmaty::fen::Fen;
use crate::model::{Game, TacticalFinding, Evaluation};

pub fn detect_tactics_for_game(game: &mut Game) {
    let moves_len = game.moves.len();
    
    for i in 0..moves_len {
        let mv = &game.moves[i];
        
        let pos_after = match mv.fen_after.parse::<Fen>() {
            Ok(fen) => match fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                Ok(pos) => pos,
                Err(_) => continue,
            },
            Err(_) => continue,
        };
        
        let mut findings = Vec::new();
        
        // Missed Mate & Missed Forced Mate
        if let Some(eval_before) = &mv.evaluation {
            if let Evaluation::Mate(m_before_ref) = eval_before {
                let m_before = *m_before_ref;
                let is_winning_mate = (mv.is_white && m_before > 0) || (!mv.is_white && m_before < 0);
                
                if is_winning_mate && !pos_after.is_checkmate() {
                    let eval_after = if i + 1 < moves_len {
                        game.moves[i + 1].evaluation.clone()
                    } else {
                        game.final_evaluation.clone()
                    };
                    
                    match eval_after {
                        Some(Evaluation::Mate(m_after)) => {
                            let mate_distance_increased = if mv.is_white {
                                m_after <= 0 || m_after > m_before
                            } else {
                                m_after >= 0 || m_after.abs() > m_before.abs()
                            };
                            
                            if mate_distance_increased {
                                if m_before.abs() == 1 {
                                    findings.push(TacticalFinding::MissedMate);
                                } else {
                                    findings.push(TacticalFinding::MissedForcedMate);
                                }
                            }
                        }
                        Some(Evaluation::Centipawns(_)) | None => {
                            if m_before.abs() == 1 {
                                findings.push(TacticalFinding::MissedMate);
                            } else {
                                findings.push(TacticalFinding::MissedForcedMate);
                            }
                        }
                    }
                }
            }
        }
        
        // Save tactical findings
        game.moves[i].tactical_findings = findings;
    }
}

pub fn detect_all_tactics(games: &mut [Game]) {
    for game in games.iter_mut() {
        detect_tactics_for_game(game);
    }
}

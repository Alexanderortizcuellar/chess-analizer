use crate::model::{Game, MoveClassification, Evaluation};

pub struct ClassificationConfig {
    pub best_move_threshold: f64,
    pub excellent_threshold: f64,
    pub good_threshold: f64,
    pub inaccuracy_threshold: f64,
    pub mistake_threshold: f64,
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        ClassificationConfig {
            best_move_threshold: 0.10,
            excellent_threshold: 0.20,
            good_threshold: 0.50,
            inaccuracy_threshold: 1.00,
            mistake_threshold: 2.00,
        }
    }
}

pub fn classify_game_moves(
    game: &mut Game,
    config: &ClassificationConfig,
) {
    let moves_len = game.moves.len();
    for i in 0..moves_len {
        let is_white = game.moves[i].is_white;
        
        // 1. If the move played matches the engine's best move in this position, it is the Best Move
        let is_engine_best = if let Some(meta) = &game.moves[i].engine_metadata {
            game.moves[i].uci == meta.best_move
        } else {
            false
        };

        if is_engine_best {
            game.moves[i].classification = Some(MoveClassification::BestMove);
            continue;
        }

        // 2. Otherwise, look at the evaluation change.
        // Eval before the move (E_before):
        let eval_before = match &game.moves[i].evaluation {
            Some(ev) => ev,
            None => continue, // Cannot classify without evaluation
        };

        // Eval after the move (E_after):
        let eval_after = if i + 1 < moves_len {
            game.moves[i + 1].evaluation.clone()
        } else {
            game.final_evaluation.clone()
        };

        let eval_after = match eval_after {
            Some(ev) => ev,
            None => continue, // Cannot classify without evaluation after
        };

        // Calculate centipawn loss
        let val_before = eval_before.absolute_value();
        let val_after = eval_after.absolute_value();

        let loss = if is_white {
            val_before - val_after
        } else {
            val_after - val_before
        };

        // 3. Check for Missed Tactic (Miss)
        let is_winning_mate_before = match eval_before {
            Evaluation::Mate(m) => (is_white && *m > 0) || (!is_white && *m < 0),
            _ => false,
        };
        let is_winning_mate_after = match &eval_after {
            Evaluation::Mate(m) => (is_white && *m > 0) || (!is_white && *m < 0),
            _ => false,
        };
        let mate_escaped = is_winning_mate_before && !is_winning_mate_after;

        let advantage_before = if is_white { val_before } else { -val_before };
        let advantage_after = if is_white { val_after } else { -val_after };

        let was_winning = advantage_before >= 1.5 || is_winning_mate_before;
        let lost_advantage = loss >= 1.0 || mate_escaped;

        if was_winning && lost_advantage {
            if advantage_after < -1.0 {
                game.moves[i].classification = Some(MoveClassification::Blunder);
            } else {
                game.moves[i].classification = Some(MoveClassification::Miss);
            }
            continue;
        }

        // 4. Classify based on thresholds
        if loss <= config.best_move_threshold {
            game.moves[i].classification = Some(MoveClassification::BestMove);
        } else if loss <= config.excellent_threshold {
            game.moves[i].classification = Some(MoveClassification::Excellent);
        } else if loss <= config.good_threshold {
            game.moves[i].classification = Some(MoveClassification::Good);
        } else if loss <= config.inaccuracy_threshold {
            game.moves[i].classification = Some(MoveClassification::Inaccuracy);
        } else if loss <= config.mistake_threshold {
            game.moves[i].classification = Some(MoveClassification::Mistake);
        } else {
            game.moves[i].classification = Some(MoveClassification::Blunder);
        }
    }
}

pub fn classify_all_moves(
    games: &mut [Game],
    config: &ClassificationConfig,
) {
    for game in games.iter_mut() {
        classify_game_moves(game, config);
    }
}

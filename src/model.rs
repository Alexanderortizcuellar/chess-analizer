use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum Evaluation {
    Centipawns(i32), // positive for White advantage, negative for Black advantage
    Mate(i32),       // positive for White mate in N, negative for Black mate in N
}

impl std::fmt::Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Evaluation::Centipawns(cp) => {
                let val = *cp as f64 / 100.0;
                write!(f, "{:+.2}", val)
            }
            Evaluation::Mate(m) => {
                if *m > 0 {
                    write!(f, "#+{}", m)
                } else {
                    write!(f, "#-{}", m.abs())
                }
            }
        }
    }
}

impl Evaluation {
    /// Return the score from the perspective of active player.
    /// In our internal model, evaluations are absolute (positive = White advantage).
    pub fn absolute_value(&self) -> f64 {
        match self {
            Evaluation::Centipawns(cp) => *cp as f64 / 100.0,
            // A mate is a massive score
            Evaluation::Mate(m) => {
                if *m >= 0 {
                    1000.0 - *m as f64
                } else {
                    -1000.0 - m.abs() as f64
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveClassification {
    BestMove,
    Excellent,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
    Brilliant,
    Miss,
}

impl std::fmt::Display for MoveClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveClassification::BestMove => write!(f, "Best Move"),
            MoveClassification::Excellent => write!(f, "Excellent"),
            MoveClassification::Good => write!(f, "Good"),
            MoveClassification::Inaccuracy => write!(f, "Inaccuracy"),
            MoveClassification::Mistake => write!(f, "Mistake"),
            MoveClassification::Blunder => write!(f, "Blunder"),
            MoveClassification::Brilliant => write!(f, "Brilliant"),
            MoveClassification::Miss => write!(f, "Miss"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TacticalFinding {
    MissedMate,
    MissedForcedMate,
    MissedFork,
    MissedSkewer,
    MissedPin,
    MissedDiscoveredAttack,
    HangingPiece,
    FreePieceNotCaptured,
    PassedPawnOpportunity,
    PromotionOpportunity,
    OnlyWinningMove,
}

impl std::fmt::Display for TacticalFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TacticalFinding::MissedMate => write!(f, "Missed Mate"),
            TacticalFinding::MissedForcedMate => write!(f, "Missed Forced Mate"),
            TacticalFinding::MissedFork => write!(f, "Missed Fork"),
            TacticalFinding::MissedSkewer => write!(f, "Missed Skewer"),
            TacticalFinding::MissedPin => write!(f, "Missed Pin"),
            TacticalFinding::MissedDiscoveredAttack => write!(f, "Missed Discovered Attack"),
            TacticalFinding::HangingPiece => write!(f, "Hanging Piece"),
            TacticalFinding::FreePieceNotCaptured => write!(f, "Free Piece Not Captured"),
            TacticalFinding::PassedPawnOpportunity => write!(f, "Passed Pawn Opportunity"),
            TacticalFinding::PromotionOpportunity => write!(f, "Promotion Opportunity"),
            TacticalFinding::OnlyWinningMove => write!(f, "Only Winning Move"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMetadata {
    pub best_move: String,          // UCI notation, e.g. "e2e4"
    pub principal_variation: Vec<String>, // SAN or UCI moves
    pub depth: u32,
    pub nodes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedMove {
    pub move_number: u32,
    pub ply: u32,                  // 0-indexed half-move count (ply)
    pub is_white: bool,
    pub san: String,               // e.g. "Nf3"
    pub uci: String,               // e.g. "g1f3"
    pub fen_before: String,
    pub fen_after: String,
    pub evaluation: Option<Evaluation>,
    pub classification: Option<MoveClassification>,
    pub tactical_findings: Vec<TacticalFinding>,
    pub engine_metadata: Option<EngineMetadata>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub headers: Vec<(String, String)>,
    pub moves: Vec<AnalyzedMove>,
    pub final_evaluation: Option<Evaluation>,
}

impl Game {
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

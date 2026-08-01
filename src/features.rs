use shakmaty::{Board, Color, Square, Bitboard, Role, File, Rank};
use shakmaty::attacks;

/// Determine if a square is attacked by a given color on a board.
pub fn is_square_attacked(board: &Board, square: Square, color: Color) -> bool {
    let occupied = board.occupied();
    let by_color = board.by_color(color);
    
    // Pawn attackers
    let pawn_attackers = attacks::pawn_attacks(color.other(), square) & board.pawns() & by_color;
    if !pawn_attackers.is_empty() {
        return true;
    }
    
    // Knight attackers
    let knight_attackers = attacks::knight_attacks(square) & board.knights() & by_color;
    if !knight_attackers.is_empty() {
        return true;
    }
    
    // Bishop / Queen attackers (sliding)
    let bishop_attackers = attacks::bishop_attacks(square, occupied) & (board.bishops() | board.queens()) & by_color;
    if !bishop_attackers.is_empty() {
        return true;
    }
    
    // Rook / Queen attackers (sliding)
    let rook_attackers = attacks::rook_attacks(square, occupied) & (board.rooks() | board.queens()) & by_color;
    if !rook_attackers.is_empty() {
        return true;
    }
    
    // King attackers
    let king_attackers = attacks::king_attacks(square) & board.kings() & by_color;
    if !king_attackers.is_empty() {
        return true;
    }
    
    false
}

/// Get the bitboard of all attackers of a square of a given color.
pub fn get_attackers(board: &Board, square: Square, color: Color) -> Bitboard {
    let occupied = board.occupied();
    let by_color = board.by_color(color);
    
    let pawn_attackers = attacks::pawn_attacks(color.other(), square) & board.pawns() & by_color;
    let knight_attackers = attacks::knight_attacks(square) & board.knights() & by_color;
    let bishop_attackers = attacks::bishop_attacks(square, occupied) & (board.bishops() | board.queens()) & by_color;
    let rook_attackers = attacks::rook_attacks(square, occupied) & (board.rooks() | board.queens()) & by_color;
    let king_attackers = attacks::king_attacks(square) & board.kings() & by_color;
    
    pawn_attackers | knight_attackers | bishop_attackers | rook_attackers | king_attackers
}

/// A piece at `square` is defended if there is at least one piece of the same color attacking `square`.
pub fn is_defended(board: &Board, square: Square, defender_color: Color) -> bool {
    is_square_attacked(board, square, defender_color)
}

/// A piece of `color` at `square` is hanging if it is attacked by the opponent and not defended,
/// OR if it is attacked by an opponent piece of lower value than itself (even if defended).
pub fn is_hanging_at(board: &Board, square: Square, color: Color) -> bool {
    let piece = match board.piece_at(square) {
        Some(p) if p.color == color => p,
        _ => return false,
    };
    
    let opponent_color = color.other();
    let attackers = get_attackers(board, square, opponent_color);
    if attackers.is_empty() {
        return false;
    }
    
    let defended = is_defended(board, square, color);
    if !defended {
        return true;
    }
    
    // If defended, check if any attacker has a lower value than this piece
    let piece_val = piece_value(piece.role);
    for sq in attackers {
        if let Some(attacker_piece) = board.piece_at(sq) {
            if piece_value(attacker_piece.role) < piece_val {
                return true;
            }
        }
    }
    
    false
}

/// Get all hanging pieces of a color on the board.
pub fn get_hanging_pieces(board: &Board, color: Color) -> Bitboard {
    let mut hanging = Bitboard::EMPTY;
    let my_pieces = board.by_color(color);
    for sq in my_pieces {
        if is_hanging_at(board, sq, color) {
            hanging |= Bitboard::from(sq);
        }
    }
    hanging
}

/// Check if a piece of `color` at `square` is pinned to the king.
pub fn is_pinned(board: &Board, square: Square, color: Color) -> bool {
    let king_sq = match board.king_of(color) {
        Some(sq) => sq,
        None => return false,
    };
    
    if square == king_sq {
        return false; // King cannot be pinned to itself
    }
    
    let opponent = color.other();
    let by_opponent = board.by_color(opponent);
    
    // Attackers of the king before removing the piece
    let occupied_before = board.occupied();
    let bishop_before = attacks::bishop_attacks(king_sq, occupied_before) & (board.bishops() | board.queens()) & by_opponent;
    let rook_before = attacks::rook_attacks(king_sq, occupied_before) & (board.rooks() | board.queens()) & by_opponent;
    
    // Attackers of the king after removing the piece
    let mut test_board = board.clone();
    test_board.discard_piece_at(square);
    
    let occupied_after = test_board.occupied();
    let bishop_after = attacks::bishop_attacks(king_sq, occupied_after) & (test_board.bishops() | test_board.queens()) & by_opponent;
    let rook_after = attacks::rook_attacks(king_sq, occupied_after) & (test_board.rooks() | test_board.queens()) & by_opponent;
    
    // A piece is pinned if removing it exposes a NEW sliding attacker to the king
    !(bishop_after & !bishop_before).is_empty() || !(rook_after & !rook_before).is_empty()
}

/// Get all pinned pieces of a color.
pub fn get_pinned_pieces(board: &Board, color: Color) -> Bitboard {
    let mut pinned = Bitboard::EMPTY;
    let my_pieces = board.by_color(color);
    for sq in my_pieces {
        if is_pinned(board, sq, color) {
            pinned |= Bitboard::from(sq);
        }
    }
    pinned
}

/// Check if a pawn of `color` at `square` is a passed pawn.
/// A pawn is passed if there are no opponent pawns in front of it on the same or adjacent files.
pub fn is_passed_pawn(board: &Board, square: Square, color: Color) -> bool {
    if !board.piece_at(square).map_or(false, |p| p.role == Role::Pawn && p.color == color) {
        return false;
    }
    
    let file = square.file();
    let rank = square.rank();
    let opponent_pawns = board.pawns() & board.by_color(color.other());
    let rank_idx = rank as u32;
    
    // Helper to check if any opponent pawn is in front of the pawn in a given file
    let has_opponent_pawn_in_front_on_file = |f| {
        for r in 0..8 {
            let sq = Square::from_coords(f, Rank::new(r));
            if opponent_pawns.contains(sq) {
                // Is this rank "in front" of the pawn?
                let is_in_front = match color {
                    Color::White => r > rank_idx,
                    Color::Black => r < rank_idx,
                };
                if is_in_front {
                    return true;
                }
            }
        }
        false
    };
    
    // Check same file
    if has_opponent_pawn_in_front_on_file(file) {
        return false;
    }
    
    let file_idx = file as i32;
    
    // Check left file
    if file_idx - 1 >= 0 {
        let left_file = File::new((file_idx - 1) as u32);
        if has_opponent_pawn_in_front_on_file(left_file) {
            return false;
        }
    }
    
    // Check right file
    if file_idx + 1 < 8 {
        let right_file = File::new((file_idx + 1) as u32);
        if has_opponent_pawn_in_front_on_file(right_file) {
            return false;
        }
    }
    
    true
}

/// Helper to get chess piece values in centipawns (roughly).
pub fn piece_value(role: Role) -> i32 {
    match role {
        Role::Pawn => 100,
        Role::Knight => 300,
        Role::Bishop => 300,
        Role::Rook => 500,
        Role::Queen => 900,
        Role::King => 20000,
    }
}

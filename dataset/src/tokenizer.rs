use anyhow::{anyhow, Ok};
use shakmaty::{uci::Uci, Bitboard, Board, Chess, Color, File, Outcome, Piece, Position, Rank, Role, Square};
use std::collections::HashMap;

/*

0 - <start:WHITE> - Start of the game (winner - white)
1 - <start:BLACK> - Start of the game (winner - black)
2 - <start:DRAW> - Start of the game (draw)
3 - <end> - End of the game

*/

#[derive(Debug)]
pub struct Token {
    value: u16,
}

pub struct Tokenizer {
    token_map: HashMap<Uci, Token>,
}

impl Tokenizer {
    pub fn new() -> Tokenizer {
        let mut tokenizer = Tokenizer { token_map: HashMap::new() };
        
        let mut idx = 4;
        for from_square in Square::ALL {
            let mut possible_moves = Bitboard::EMPTY;
            for role in Role::ALL {
                let mut board = Board::empty();
                board.set_piece_at(from_square, Piece { color: Color::White, role: role });
                possible_moves = possible_moves | board.attacks_from(from_square);
            }

            for to_square in possible_moves {
                let uci = Uci::Normal { from: from_square, to: to_square, promotion: None };
                tokenizer.token_map.insert(uci, Token { value: idx });
                idx += 1;
            }

            match from_square {
                Square::A7 | Square::B7 | Square::C7 | Square::D7 | Square::E7 | Square::F7 | Square::G7 | Square::H7 => {
                    let mut possible_moves = Bitboard::EMPTY;
                    for role in [Role::Rook, Role::Bishop] {
                        let mut board = Board::empty();
                        board.set_piece_at(from_square, Piece { color: Color::White, role: role });
                        possible_moves = possible_moves | board.attacks_from(from_square);
                    }
                    possible_moves = possible_moves & Bitboard::BACKRANKS & Bitboard::NORTH;
                    
                    for to_square in possible_moves {
                        for promotion in [Role::Bishop, Role::Knight, Role::Rook, Role::Queen] {
                            let uci = Uci::Normal { from: from_square, to: to_square, promotion: Some(promotion) };
                            tokenizer.token_map.insert(uci, Token { value: idx });
                            idx += 1;
                        }
                    }
                },
                _ => {},
            }

            match from_square {
                Square::A2 | Square::B2 | Square::C2 | Square::D2 | Square::E2 | Square::F2 | Square::G2 | Square::H2 => {
                    let mut possible_moves = Bitboard::EMPTY;
                    for role in [Role::Rook, Role::Bishop] {
                        let mut board = Board::empty();
                        board.set_piece_at(from_square, Piece { color: Color::White, role: role });
                        possible_moves = possible_moves | board.attacks_from(from_square);
                    }
                    possible_moves = possible_moves & Bitboard::BACKRANKS & Bitboard::SOUTH;
                    
                    for to_square in possible_moves {
                        for promotion in [Role::Bishop, Role::Knight, Role::Rook, Role::Queen] {
                            let uci = Uci::Normal { from: from_square, to: to_square, promotion: Some(promotion) };
                            tokenizer.token_map.insert(uci, Token { value: idx });
                            idx += 1;
                        }
                    }
                },
                _ => {},
            }
        }

        println!("{:?}", tokenizer.token_map.capacity());

        return tokenizer;
    }

    pub fn game_start_token(self, outcome: Option<Outcome>) -> Result<Token, anyhow::Error> {
        return match outcome {
            Some(outcome) => match outcome {
                Outcome::Decisive { winner } => match winner {
                    Color::White => Ok(Token { value: 0 }),
                    Color::Black => Ok(Token { value: 1 }),
                },
                Outcome::Draw => Ok(Token { value: 2 }),
            },
            None => Err(anyhow!("Outcome not specified")),
        };
    }

    pub fn uci_to_token(self, uci: Uci) -> Result<Token, anyhow::Error> {
        if self.token_map.contains_key(&uci) {
            return Ok(Token { value: self.token_map[&uci].value });
        }

        return Err(anyhow!("Invalid uci string found"));
    }

    pub fn game_end_token(self) -> Token {
        return Token { value: 3 };
    }
}

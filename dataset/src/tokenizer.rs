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

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

pub struct Tokenizer {
    token_map: HashMap<Uci, Token>,
}

impl Tokenizer {
    pub fn new() -> Tokenizer {
        let mut tokenizer = Tokenizer { token_map: HashMap::new() };
        
        

        println!("{:?}", tokenizer.token_map.capacity());

        return tokenizer;
    }

    pub fn initialize_token_map(&mut self) {
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
                self.token_map.insert(uci, Token { value: idx });
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
                            self.token_map.insert(uci, Token { value: idx });
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
                            self.token_map.insert(uci, Token { value: idx });
                            idx += 1;
                        }
                    }
                },
                _ => {},
            }
        }
    }

    pub fn game_start_token(&self, outcome: Option<Outcome>) -> Result<Token, anyhow::Error> {
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

    pub fn uci_to_token(&mut self, uci: Uci) -> Result<Token, anyhow::Error> {
        if self.token_map.len() == 0 { self.initialize_token_map(); } 

        if self.token_map.contains_key(&uci) {
            return Ok(Token { value: self.token_map[&uci].value });
        }

        return Err(anyhow!("Invalid uci string found"));
    }

    pub fn game_end_token(&self) -> Token {
        return Token { value: 3 };
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Read, str::FromStr};

    use super::*;

    #[test]
    fn game_start_token() {
        let tokenizer = Tokenizer::new();
        assert_eq!(tokenizer.game_start_token(Option::Some(Outcome::Decisive { winner: Color::White })).unwrap(), Token { value: 0 });
        assert_eq!(tokenizer.game_start_token(Option::Some(Outcome::Decisive { winner: Color::Black })).unwrap(), Token { value: 1 });
        assert_eq!(tokenizer.game_start_token(Option::Some(Outcome::Draw)).unwrap(), Token { value: 2 });
        assert_eq!(tokenizer.game_start_token(Option::None).unwrap_err().to_string(), "Outcome not specified");
    }

    #[test]
    fn uci_to_token() {
        let mut tokenizer = Tokenizer::new();
        tokenizer.initialize_token_map();
        
        let mut uci_strings = String::new();
        std::fs::File::read_to_string(&mut std::fs::File::open("assets/uci_strings.txt").unwrap(), &mut uci_strings).expect("Unexpected error occured");

        assert_eq!(uci_strings.lines().count(), tokenizer.token_map.len());
        for uci in uci_strings.lines() {
            let uci = Uci::from_str(uci).unwrap();
            assert!(tokenizer.token_map.contains_key(&uci));
        }
    }

    #[test]
    fn game_end_token() {
        let tokenizer = Tokenizer::new();
        assert_eq!(tokenizer.game_end_token(), Token { value: 3 });
    }
}

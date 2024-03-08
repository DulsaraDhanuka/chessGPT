use anyhow::{anyhow, Error};
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use shakmaty::{Chess, Move, Outcome, Position};

use crate::tokenizer::Token;

pub trait PgnVisitor {
    fn begin_game(&mut self, _outcome: Outcome) -> Result<Vec<Token>, Error> { return Err(anyhow!("Not implemented")); }
    fn game_move(&mut self, _move: Move) -> Result<Vec<Token>, Error> { return Err(anyhow!("Not implemented")); }
    fn end_game(&mut self) -> Result<Vec<Token>, Error> { return Err(anyhow!("Not implemented")); }

    fn save_game(&mut self, _game: Vec<Token>) -> Result<(), Error> { return Err(anyhow!("Not implemented")); } 
}

struct OrigPgnVisitor<'a, V: PgnVisitor> {
    visitor: &'a mut V,
    skip_current_game: bool,
    current_game_moves: Vec<Move>,
    current_game_outcome: Outcome,
    current_pos: Chess,
}

impl<V: PgnVisitor> OrigPgnVisitor<'_, V> {
    fn new(visitor: &mut V) -> OrigPgnVisitor<V> {
        return OrigPgnVisitor { 
            skip_current_game: false,
            current_game_moves: Vec::new(),
            current_game_outcome: Outcome::Draw, 
            visitor: visitor, 
            current_pos: Chess::default(),
        };

    }
}

impl<V: PgnVisitor> Visitor for OrigPgnVisitor<'_, V> {
    type Result = bool;

    fn begin_game(&mut self) { }

    fn san(&mut self, san_plus: SanPlus) {
        match san_plus.san.to_move(&self.current_pos) {
            Ok(_move) => {
                match self.current_pos.clone().play(&_move) {
                    Ok(pos) => {
                        self.current_pos = pos;
                        self.current_game_moves.push(_move);
                    },
                    Err(e) => {
                        self.skip_current_game = true;
                        println!("Error: {}", e);
                    },
                }
            },
            Err(e) => {
                self.skip_current_game = true;
                println!("Error: {}", e);
            },
        }
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        self.current_game_outcome = match outcome {
            Some(outcome) => outcome,
            None => {
                self.skip_current_game = true;
                Outcome::Draw
            },
        };
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        let mut error = false;
        if !self.skip_current_game {
            let mut current_game: Vec<Token> = Vec::new();
            if !error {
                match self.visitor.begin_game(self.current_game_outcome) {
                    Ok(pgn) => current_game.extend(pgn),
                    Err(e) => {
                        error = true;
                        println!("Error: {}", e);
                    }
                }
            }

            if !error {
                for _move in self.current_game_moves.clone() {
                    match self.visitor.game_move(_move) {
                        Ok(pgn) => current_game.extend(pgn),
                        Err(e) => {
                            error = true;
                            println!("Error: {}", e);
                        }
                    }
                }
            }

            if !error {
                match self.visitor.end_game() {
                    Ok(pgn) => current_game.extend(pgn),
                    Err(e) => {
                        error = true;
                        println!("Error: {}", e);
                    }
                }
            }

            if !error {
                match self.visitor.save_game(current_game) {
                    Ok(_) => {},
                    Err(e) => {
                        error = true;
                        println!("Error: {}", e);
                    }
                }
            }
        }

        self.skip_current_game = false;
        self.current_game_outcome = Outcome::Draw;
        self.current_game_moves = Vec::new();
        self.current_pos = Chess::default();
        return !(error || self.skip_current_game);
    }
}

pub fn visit_games_from_pgn_string<V: PgnVisitor>(pgn_string: String, visitor: &mut V) -> Result<(), Error> {
    let mut pgn_buffer = BufferedReader::new_cursor(pgn_string);
    let mut orig_visitor = OrigPgnVisitor::new(visitor);
    return match pgn_buffer.read_all(&mut orig_visitor) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("Error parsing PGN: {}", e)),
    };
}

use std::sync::{Arc, Mutex};
//use std::thread::{self, JoinHandle};
use std::fs;
use std::io::Write;
use clap::Parser;

use anyhow::Error;
use pgn_parser::PgnVisitor;
use shakmaty::{Color, Move, Outcome};
use tokenizer::{Token, Tokenizer};
use std::collections::HashSet;

mod utils;
mod tokenizer;
mod pgn_reader;
mod pgn_parser;

struct Visitor<'a> {
    output: Vec<u8>,
    tokenizer: &'a Tokenizer,
    hash_collection: Arc<Mutex<HashSet<String>>>,
    game_indexes: Vec<u64>,
    current_outcome: Option<Outcome>,
    current_ply: u32,

    white_winning_games: u32,
    black_winning_games: u32,
    draw_games: u32,
    duplicate_games: u32,
    ply: u32,
}

impl Visitor<'_> {
    fn new<'a>(tokenizer: &'a Tokenizer, hash_collection: Arc<Mutex<HashSet<String>>>) -> Visitor<'a> {
        Visitor {
            output: Vec::new(),
            tokenizer,
            hash_collection,
            game_indexes: Vec::new(),
            current_outcome: Option::None,
            current_ply: 0,

            white_winning_games: 0,
            black_winning_games: 0,
            draw_games: 0,
            duplicate_games: 0,
            ply: 0,
        }
    }
}

impl PgnVisitor for Visitor<'_> {
    fn begin_game(&mut self, _outcome: Outcome) -> Result<Vec<Token>, Error> { 
        match self.tokenizer.game_start_token(Option::Some(_outcome)) {
            Ok(v) => {
                self.current_outcome = Some(_outcome);
                Ok(Vec::from([v]))
            },
            Err(e) => Err(e),
        }
    }

    fn game_move(&mut self, _move: Move) -> Result<Vec<Token>, Error> {
        match self.tokenizer.uci_to_token(_move.to_uci(shakmaty::CastlingMode::Standard)) {
            Ok(v) => {
                self.current_ply += 1;
                Ok(Vec::from([v]))
            },
            Err(e) => Err(e),
        }
    }

    fn end_game(&mut self) -> Result<Vec<Token>, Error> {
        Ok(Vec::from([self.tokenizer.game_end_token()]))
    }

    fn save_game(&mut self, _game: Vec<Token>) -> Result<(), Error> {
        match self.hash_collection.lock() {
            Ok(mut hash_collection) => {
                let mut game: Vec<u8> = Vec::new();
                for tok in _game {
                    game.extend(tok.value.to_be_bytes());
                }

                let hash = sha256::digest(game.clone());
                
                if !hash_collection.contains(&hash) {
                    hash_collection.insert(hash);
                    self.game_indexes.push(self.output.len() as u64);

                    self.output.extend(game);
                    match self.current_outcome {
                        Some(outcome) => match outcome {
                            Outcome::Decisive { winner } => match winner {
                                Color::White => self.white_winning_games += 1,
                                Color::Black => self.black_winning_games += 1,
                            },
                            Outcome::Draw => self.draw_games += 1,
                        },
                        None => {}
                    }
                    self.ply += self.current_ply;
                } else {
                    // Duplicate game!!!
                    self.duplicate_games += 1;
                }
            },
            Err(e) => println!("Error: {e}"),
        }

        self.current_ply = 0;
        self.current_outcome = Option::None;

        Ok(())
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {

    #[arg(long, help="Input path of a json containing input file urls")]
    input: String,

    #[arg(long, help="Output path of the save file")]
    output: String,

    #[arg(long, help="Encoder save path")]
    encoder_output: String,

    #[arg(long, help="Index save path")]
    index_output: String,

    #[arg(long, help="Number of worker threads")]
    n_workers: u16,
}

struct Stats {
    white_winning_games: u32,
    black_winning_games: u32,
    draw_games: u32,
    duplicate_games: u32,
    ply: u32,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            white_winning_games: 0,
            black_winning_games: 0,
            draw_games: 0,
            duplicate_games: 0,
            ply: 0,
        }
    }
}

fn main() {
    let args = Args::parse();

    let pool = rayon::ThreadPoolBuilder::new().num_threads(args.n_workers as usize).build().unwrap();

    let mut tokenizer = tokenizer::Tokenizer::new();
    tokenizer.create_token_map();
    tokenizer.save(&args.encoder_output);
    let tokenizer = Arc::new(tokenizer);

    let stats = Arc::new(Mutex::new(Stats::new()));

    let hash_collection = Arc::new(Mutex::new(HashSet::<String>::new()));
    let global_game_idx = Arc::new(Mutex::new(u64::MIN));

    let game_indexes_file = Arc::new(Mutex::new(fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&args.index_output).expect("Error occured while creating index output file")));

    match utils::read_urls_from_input_json(args.input) {
        Ok(urls) => {            
            let file = Arc::new(Mutex::new(fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&args.output).expect("Error occured while creating output file")));

            for url in urls {
                let file = Arc::clone(&file);
                let game_indexes_file = Arc::clone(&game_indexes_file);
                let tokenizer = Arc::clone(&tokenizer);
                let stats = Arc::clone(&stats);
                let hash_collection = Arc::clone(&hash_collection);
                let global_game_idx = Arc::clone(&global_game_idx);

                pool.install(move || {
                    match pgn_reader::download_bytes_from_url(url.clone()) {
                        Ok(content) => {
                            match pgn_reader::pgn_string_from_bytes(url.clone(), content) {
                                Ok(pgn_string) => {
                                    let mut visitor = Visitor::new(&tokenizer, hash_collection);
                                    match pgn_parser::visit_games_from_pgn_string(pgn_string, &mut visitor) {
                                        Ok(_) => {
                                            match file.lock() {
                                                Ok(mut file) => {
                                                    match file.write_all(&visitor.output) {
                                                        Ok(_) => {
                                                            match global_game_idx.lock() {
                                                                Ok(mut global_game_idx) => {                                                                    
                                                                    match game_indexes_file.lock() {
                                                                        Ok(mut game_indexes_file) => {
                                                                            let mut indexes = Vec::<u8>::new();
                                                                            for idx in visitor.game_indexes {
                                                                                indexes.extend((idx + *global_game_idx).to_be_bytes());
                                                                            }
                                                                            match game_indexes_file.write_all(&indexes) {
                                                                                Ok(_) => *global_game_idx += visitor.output.len() as u64,
                                                                                Err(e) => panic!("Unable to write to file, {}, Error: {}", &url, e),
                                                                            }
                                                                        },
                                                                        Err(e) => panic!("Unable to lock game_indexes_file, {}, Error: {}", &url, e),
                                                                    }
                                                                },
                                                                Err(e) => panic!("Unable to lock global_game_idx, {}, Error: {}", &url, e),
                                                            }

                                                            match stats.lock() {
                                                                Ok(mut stats) => {
                                                                    stats.white_winning_games += visitor.white_winning_games;
                                                                    stats.black_winning_games += visitor.black_winning_games;
                                                                    stats.draw_games += visitor.draw_games;
                                                                    stats.duplicate_games += visitor.duplicate_games;
                                                                    stats.ply += visitor.ply;
                                                                    let total_games = visitor.white_winning_games + visitor.black_winning_games + visitor.draw_games;
                                                                    println!("Games: {:0width$}, Dup games: {:0width$}, Ply: {:0width$} - {}", total_games, visitor.duplicate_games, visitor.ply, url, width=15);
                                                                },
                                                                Err(e) => println!("Error: {}", e),
                                                            }
                                                        },
                                                        Err(e) => panic!("Unable to write to file, {}, Error: {}", &url, e),
                                                    }
                                                },
                                                Err(e) => panic!("Unable to lock file, {}, Error: {}", &url, e),
                                            }
                                        },
                                        Err(e) => println!("Error: {}", e),
                                    }
                                },
                                Err(e) => {println!("Error: {} - {}", &url, e)},
                            };        
                        },
                        Err(e) => println!("Error: {} - {}", &url, e),
                    };
                });
            }
        },
        Err(e) => panic!("Error: {}", e)
    }

    let stats = stats.lock().unwrap();
    println!("Total white winning games - {}", stats.white_winning_games);
    println!("Total black winning games - {}", stats.black_winning_games);
    println!("Total drawn games         - {}", stats.draw_games);
    println!("Total duplicate games     - {}", stats.duplicate_games);
    println!("Total plys                - {}", stats.ply);

    /*

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("output.games").unwrap();

    let url = "https://www.pgnmentor.com/events/Tilburg1981.pgn";
    match pgn_reader::download_bytes_from_url(String::from(url)) {
        Ok(content) => {
            match pgn_reader::pgn_string_from_bytes(String::from(url), content) {
                Ok(pgn_string) => {
                    let mut visitor = Visitor::new(&tokenizer);
                    match pgn_parser::visit_games_from_pgn_string(pgn_string, &mut visitor) {
                        Ok(_) => {
                            match file.write_all(&visitor.output) {
                                Ok(_) => {
                                    println!("Done {} - {}", url, visitor.total_games);
                                },
                                Err(e) => println!("Error: {}", e),
                            }
                        },
                        Err(e) => println!("Error: {}", e),
                    }
                },
                Err(e) => println!("Error: {}", e),
            };        
        },
        Err(e) => println!("Error: {}", e),
    };

    //println!("{}", pgn_data);*/
}


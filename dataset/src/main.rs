use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::fs;
use std::io::Write;
use clap::Parser;

use anyhow::Error;
use pgn_parser::PgnVisitor;
use shakmaty::{Move, Outcome};
use tokenizer::{Token, Tokenizer};

mod utils;
mod tokenizer;
mod pgn_reader;
mod pgn_parser;

struct Visitor<'a> {
    output: Vec<u8>,
    total_games: u32,
    total_ply: u32,
    tokenizer: &'a Tokenizer,
}

impl Visitor<'_> {
    fn new(tokenizer: &Tokenizer) -> Visitor {
        Visitor {
            output: Vec::new(),
            total_games: 0,
            total_ply: 0,
            tokenizer,
        }
    }
}

impl PgnVisitor for Visitor<'_> {
    fn begin_game(&mut self, _outcome: Outcome) -> Result<Vec<Token>, Error> { 
        match self.tokenizer.game_start_token(Option::Some(_outcome)) {
            Ok(v) => Ok(Vec::from([v])),
            Err(e) => Err(e),
        }
    }

    fn game_move(&mut self, _move: Move) -> Result<Vec<Token>, Error> {
        match self.tokenizer.uci_to_token(_move.to_uci(shakmaty::CastlingMode::Standard)) {
            Ok(v) => {
                self.total_ply += 1;
                Ok(Vec::from([v]))
            },
            Err(e) => Err(e),
        }
    }

    fn end_game(&mut self) -> Result<Vec<Token>, Error> {
        Ok(Vec::from([self.tokenizer.game_end_token()]))
    }

    fn save_game(&mut self, _game: Vec<Token>) -> Result<(), Error> {
        for tok in _game {
            self.output.extend(tok.value.to_be_bytes());
        }
        self.total_games += 1;
        Ok(())
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {

    #[arg(short, long, help="Input path of a json containing input file urls")]
    input: String,

    #[arg(short, long, help="Output path of the save file")]
    output: String,

    #[arg(short, long, help="Encoder save path")]
    encoder_output_path: String,
}

fn main() {
    let args = Args::parse();

    let mut tokenizer = tokenizer::Tokenizer::new();
    tokenizer.create_token_map();
    tokenizer.save(&args.encoder_output_path);
    let tokenizer = Arc::new(tokenizer);

    match utils::read_urls_from_input_json(args.input) {
        Ok(urls) => {            
            let file = Arc::new(Mutex::new(fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&args.output).expect("Error occured while creating output file")));

            let mut threads: Vec<JoinHandle<()>> = Vec::new();
            for url in urls {
                let file = Arc::clone(&file);
                let tokenizer = Arc::clone(&tokenizer);
                let thread = thread::spawn(move || {
                    match pgn_reader::download_bytes_from_url(url.clone()) {
                        Ok(content) => {
                            match pgn_reader::pgn_string_from_bytes(url.clone(), content) {
                                Ok(pgn_string) => {
                                    let mut visitor = Visitor::new(&tokenizer);
                                    match pgn_parser::visit_games_from_pgn_string(pgn_string, &mut visitor) {
                                        Ok(_) => {
                                            match file.lock() {
                                                Ok(mut file) => {
                                                    match file.write_all(&visitor.output) {
                                                        Ok(_) => {
                                                            
                                                            println!("Done {} - Games: {}, Ply: {}", url, visitor.total_games, visitor.total_ply);
                                                        },
                                                        Err(e) => println!("Error: {}", e),
                                                    }
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
                });
                threads.push(thread);
            }

            for thread in threads {
                thread.join().unwrap();
            }
        },
        Err(e) => panic!("Error: {}", e)
    }

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


use anyhow::{Result, anyhow};
use std::io::Read;
use bytes::Bytes;
use bytes::Buf;
use pgn_reader::{Visitor, Skip, BufferedReader, SanPlus, Outcome, Square, Color};
use std::fs::OpenOptions;
use std::io::Write;
use std::{str, thread, env};
use shakmaty::{Chess, Position, File, Rank};
use std::sync::{Arc, Mutex};
use serde_json;

struct PgnFile {
    data: Vec<u8>,
    skip_current_game: bool,
    current_game: Vec<u8>,
    current_pos: Chess,
    total_games: u32,
}

impl PgnFile {
    fn new() -> PgnFile {
        return PgnFile { data: Vec::new(), skip_current_game: false, current_game: Vec::new(), current_pos: Chess::default(), total_games: 0 };
    }
}

fn convert_square_to_byte(square: Square) -> u8 {
    let (file, rank) = square.coords();
    let rankoffset: u8 = match rank {
        Rank::First => 0,
        Rank::Second => 1,
        Rank::Third => 2,
        Rank::Fourth => 3,
        Rank::Fifth => 4,
        Rank::Sixth => 5,
        Rank::Seventh => 6,
        Rank::Eighth => 7,
    };
    let fileoffset: u8 = match file {
        File::A => 1,
        File::B => 2,
        File::C => 3,
        File::D => 4,
        File::E => 5,
        File::F => 6,
        File::G => 7,
        File::H => 8,
    };
    
    return fileoffset + (rankoffset * 8);
}

impl Visitor for PgnFile {
    type Result = bool;

    fn begin_game(&mut self) {
        self.current_game.push(0x00);
        self.current_game.push(0x00);
    }

    fn san(&mut self, san_plus: SanPlus) {
        if let Ok(m) = san_plus.san.to_move(&self.current_pos) {
            self.current_pos.play_unchecked(&m);
            self.current_game.push(match m.from() {
                Some(x) => convert_square_to_byte(x),
                _ => {
                    self.skip_current_game = true;
                    0x01
                },
            });
            self.current_game.push(convert_square_to_byte(m.to()));
        }
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        self.current_game[1] = match outcome {
            Some(x) => match x {
                Outcome::Decisive { winner } => match winner {
                    Color::White => 0xFE,
                    Color::Black => 0xFF
                },
                Outcome::Draw => 0xFF
            },
            _ => {
                self.skip_current_game = true;
                0x00
            }
        };
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        self.current_game.push(0x00);
        let append: bool = if let Some(x) = self.current_game.get(1) { *x != 0x00 } else { false }; 
        if append && !self.skip_current_game {
            self.total_games += 1;
            self.data.append(&mut self.current_game);
        }

        self.current_game = Vec::new();
        self.current_pos = Chess::default();
        return true;
    }
}

fn parse_pgn(pgn_data: String) -> Option<PgnFile> {
    let mut pgn_buffer = BufferedReader::new_cursor(pgn_data);
    let mut game = PgnFile::new();
    let result = pgn_buffer.read_all(&mut game);
    
    match result {
        Err(e) => {
            println!("Unexpected error {:?}", e);
            return None;
        },
        _ => {}
    }

    return Some(game);
}

fn convert_from_url(url: &str, output_file: &mut std::fs::File) -> Result<()> {
    let response = reqwest::blocking::get(url)?;
    let content = Bytes::from(response.bytes()?);
    let mut pgn_data = String::new();

    if url.ends_with(".zip") {
        let mut zip_reader: Box<dyn Read> = Box::new(content.reader()) as Box<dyn Read>; 
        match zip::read::read_zipfile_from_stream(&mut zip_reader) {
            Ok(Some(mut file)) => {
                file.read_to_string(&mut pgn_data).expect("Unable to read pgn data");
                let game = parse_pgn(pgn_data);
                match game {
                    Some(x) => {
                        println!("{:?}", x.total_games);
                        match output_file.write_all(&x.data) {
                            Err(e) => {
                                println!("Error: {:?}", e);
                                return Err(e.into());
                            },
                            _ => {}
                        }
                    },
                    _ => {
                        println!("Failed to parse game.");
                    },
                }
            }
            Ok(None) => {
                println!("Error encountered while reading zip");
                return Err(anyhow!("Error encountered while reading zip"));
            },
            Err(e) => {
                println!("Error encountered while reading zip: {e:?}");
                return Err(e.into());
            }
        };
    } else if url.ends_with(".pgn") {
        pgn_data = match str::from_utf8(&content) {
            Ok(v) => String::from(v),
            Err(e) => {
                println!("Invalid UTF-8 sequence: {}", e);
                return Err(e.into());
            },
        };
        let game = parse_pgn(pgn_data);
        match game {
            Some(x) => {
                println!("{:?}", x.total_games);
                match output_file.write_all(&x.data) {
                    Err(e) => {
                        println!("Error: {:?}", e);
                        return Err(e.into());
                    },
                    _ => {}
                }
            },
            _ => println!("Unexpected error."),
        }
    } else {
        println!("Error unexpected file type.");
        return Err(anyhow!("Error unexpected file type."));
    }

    return Ok(());
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let input_filepath = &args[1];
    let output_filepath = &args[2];

    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(input_filepath)?).expect("JSON was not well-formatted");

    let file = Arc::new(Mutex::new(OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_filepath)?));

    let mut threads = Vec::new();
    match json {
        serde_json::Value::Array(values) => {
            for value in values {
                match value {
                    serde_json::Value::String(url) => {                
                        let file = Arc::clone(&file);
                        let thread = thread::spawn(move || {
                            let mut file = file.lock().unwrap();
                            match convert_from_url(&url, &mut file) {
                                Err(e) => println!("Unexpected error {:?}", e),
                                _ => {}
                            }
                        });
                        threads.push(thread);
                    },
                    _ => panic!("Unexpected input files")
                }
            }
        },
        _ => panic!("Unexpected input files"),
    }

    for thread in threads {
        thread.join().unwrap();
    }
    //convert_from_url("https://www.pgnmentor.com/players/Morphy.zip", &mut file).await?;

    return Ok(());
}


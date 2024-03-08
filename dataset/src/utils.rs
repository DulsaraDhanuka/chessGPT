use std::{fs, io::Read};

use anyhow::{anyhow, Error};

pub fn read_urls_from_input_json(input_path: String) -> Result<Vec<String>, Error> {
    match fs::OpenOptions::new().read(true).open(input_path) {
        Ok(mut input_file) => {
            let mut buf = String::new();
            match input_file.read_to_string(&mut buf) {
                Ok(_) => {
                    match serde_json::from_str(&buf) {
                        Ok(json) => {
                            match json {
                                serde_json::Value::Array(values) => {
                                    let mut urls: Vec<String> = Vec::new();
                                    for value in values {
                                        urls.push(match value {
                                            serde_json::Value::String(value)  => value,
                                            _ => return Err(anyhow!("Invalid JSON data")),
                                        });
                                    }
                                    
                                    return Ok(urls)
                                },
                                _ => return Err(anyhow!("Invalid JSON data")),
                            }
                        },
                        Err(e) => return Err(e.into()),
                    }
                },
                Err(e) => return Err(e.into()),
            }
        },
        Err(e) => return Err(e.into()),
    };
}
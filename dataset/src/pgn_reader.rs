use std::{io::Read, time::Duration};
use anyhow::{anyhow, Error};
use bytes::{Bytes, Buf};

pub fn download_bytes_from_url(url: String) -> Result<Bytes, Error> {
    let client = reqwest::blocking::ClientBuilder::new().timeout(Duration::from_secs(600)).build()?;
    let response = client.get(url).send();
    
    return match response {
        Ok(response) => {
            let body = response.bytes();
            match body {
                Ok(body) => Ok(body),
                Err(err) => Err(err.into())
            }
        },
        Err(err) => Err(err.into())
    };
}

pub fn pgn_string_from_bytes(url: String, bytes: Bytes) -> Result<String, Error> {
    return if url.ends_with(".zip") {
        pgn_string_from_bytes_zip(bytes)
    } else if url.ends_with(".pgn") {
        pgn_string_from_bytes_pgn(bytes)
    } else {
        Err(anyhow!("Unsupported file type"))
    };
}

pub fn pgn_string_from_bytes_pgn(bytes: Bytes) -> Result<String, Error> {
    let pgn_string = String::from_utf8(bytes.to_vec());
    return match pgn_string {
        Ok(pgn_string) => Ok(pgn_string),
        Err(e) => Err(e.into()),
    };
}

pub fn pgn_string_from_bytes_zip(bytes: Bytes) -> Result<String, Error> {
    let mut pgn_data = String::new();
    let mut zip_reader: Box<dyn Read> = Box::new(bytes.reader()) as Box<dyn Read>; 
    return match zip::read::read_zipfile_from_stream(&mut zip_reader) {
        Ok(Some(mut file)) => {
            match file.read_to_string(&mut pgn_data) {
                Ok(_) => Ok(pgn_data),
                Err(e) => Err(e.into()),
            }
        }
        Ok(None) => Err(anyhow!("No files found in zip archive")),
        Err(e) => Err(e.into())
    };
}

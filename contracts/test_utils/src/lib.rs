use near_sdk::serde::de::DeserializeOwned;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

pub fn file_as_json<T: DeserializeOwned>(filename: &str) -> Result<T, Box<dyn Error>> {
    let file = File::open(format!("./tests/{}", filename))?;
    let reader = BufReader::new(file);
    let value = near_sdk::serde_json::from_reader(reader)?;

    return Ok(value);
}

#![allow(non_snake_case)]
//! SEED fills out redis with data for all the special features and then
//! adds shouts from files listed as arguments.
//! Example usage: `SEED SEEDS custom.txt`
//! All seed files must be newline-delimited text files.
use anyhow::{Context, Result};
use dotenv::dotenv;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

// Message store + other data
fn seed_from_file(
    db: &mut redis::Connection,
    filename: impl AsRef<Path> + std::fmt::Debug + Copy,
    key: &str,
    skip_loud_check: bool,
) -> Result<u32, redis::RedisError> {
    let fp = File::open(filename);
    if fp.is_err() {
        println!("Skipping {:?}; could not open file for reading.", filename);
        return Ok(0);
    }

    let punc = Regex::new(r"[\W\d[[:punct:]]]").unwrap();
    let mut pipe = redis::pipe();
    let file = fp.unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let text = line.unwrap();
        let result = punc.replace_all(&text, "");
        if !skip_loud_check && (result.len() < 3 || result.to_uppercase() != result) {
            continue;
        }
        pipe.sadd(key, text);
    }

    let result: Vec<u32> = pipe.query(db)?;
    let count = result.iter().sum();
    println!("Added {} items from {:#?}", count, filename);
    Ok(count)
}

fn main() -> Result<()> {
    dotenv().ok();

    let redis_uri = match env::var("REDIS_URL") {
        Ok(v) => v,
        Err(_) => "redis://127.0.0.1:6379".to_string(),
    };
    let redis_prefix = match env::var("REDIS_PREFIX") {
        Ok(v) => v,
        Err(_) => "LB".to_string(),
    };
    let client = redis::Client::open(redis_uri.as_ref())
        .with_context(|| format!("Unable to create redis client @ {}", redis_uri))?;
    let mut rcon = client
        .get_connection()
        .with_context(|| format!("Unable to connect to redis @ {}", redis_uri))?;

    println!("Saving seed data to redis @ {redis_uri}");

    let catkey = format!("{}:CAT", redis_prefix);
    let malckey = format!("{}:MALC", redis_prefix);
    let shipkey = format!("{}:SHIPS", redis_prefix);
    let swkey = format!("{}:SW", redis_prefix);
    let yellkey = format!("{}:YELLS", redis_prefix);

    seed_from_file(&mut rcon, "CATS", &catkey, true)
        .with_context(|| "Trying to write to redis failed utterly.")?;
    seed_from_file(&mut rcon, "STAR_FIGHTING", &swkey, true)?;
    seed_from_file(&mut rcon, "SHIPS", &shipkey, true)?;
    seed_from_file(&mut rcon, "MALCOLM", &malckey, true)?;

    for f in std::env::args().skip(1) {
        seed_from_file(&mut rcon, &f, &yellkey, false)?;
    }

    Ok(())
}

#![allow(non_snake_case)]
//! Takes a list of input files containing newline-delimited text.
//! For each line in each text file, removes the item from the yell
//! set in redis.
use anyhow::{Context, Result};
use dotenv::dotenv;
use redis::Commands;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

type RCount = std::result::Result<u32, redis::RedisError>;

fn prune_from_file(
    db: &mut redis::Connection,
    filename: impl AsRef<Path> + std::fmt::Debug + Copy,
    key: &str,
) -> Result<u32, redis::RedisError> {
    let mut count: u32 = 0;
    let file = File::open(filename).expect("no such file");
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let text = line.unwrap();
        if text.len() < 2 {
            continue;
        }
        let res: RCount = db.srem(key, text);
        match res {
            Err(e) => println!("{:?}", e),
            Ok(i) => {
                count += i;
            }
        }
    }
    println!("Removed {} items from {:#?}", count, filename);
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

    let yellkey = format!("{}:YELLS", redis_prefix);

    for f in std::env::args().skip(1) {
        prune_from_file(&mut rcon, &f, &yellkey)
            .with_context(|| "Trying to write to redis failed utterly.")?;
    }

    Ok(())
}

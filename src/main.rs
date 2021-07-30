#[macro_use]
extern crate anyhow;
extern crate crossbeam;
extern crate num_cpus;
#[macro_use]
extern crate lazy_static;
extern crate reqwest;

use anyhow::Result;
use crossbeam::channel::bounded;
use crossbeam::channel::{Receiver, Sender};
use reqwest::{blocking::Client, header::HeaderMap};
use std::env;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::thread;
use std::time::Duration;

type Token = String;
type Validated = Option<String>;

fn main() -> Result<()> {
    // only run the program if we have both an input and output file
    let usage = "usage: tokval input_file.txt output_file.txt";
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("{}", usage);
        return Ok(());
    }

    // open input file and ensure it's good
    let input_file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(args.get(1).unwrap());
    let input_file = match input_file {
        Ok(f) => f,
        Err(e) => {
            println!("error opening input file [{}]: {}", args[1], e);
            return Err(anyhow!(e));
        }
    };

    // open output file and ensure it's good
    let output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(args.get(2).unwrap());
    let output_file = match output_file {
        Ok(f) => f,
        Err(e) => {
            println!("error opening output file: [{}]: {}", args[2], e);
            return Err(anyhow!(e));
        }
    };

    // spawn as many threads as the cpu has
    let num_threads = num_cpus::get();
    // create the initial senders and receivers
    let (tok_send, tok_recv) = bounded::<Token>(1);
    let (val_send, val_recv) = bounded::<Validated>(1);

    // load all tokens in from the input file
    let mut tokens: Vec<String> = Vec::new();
    for line in BufReader::new(input_file).lines() {
        let line = line?;
        if !line.trim().is_empty() {
            tokens.push(line.trim().to_string());
        }
    }

    println!("read [{}] tokens", tokens.len());
    println!("spawning [{}] worker threads", num_threads);

    let total_tokens = tokens.len();
    let mut num_validated = 0;

    // crossbeam scope where all worker threads are created
    crossbeam::scope(|sc| {
        // first, spawn a single thread to send tokens
        sc.spawn(|_| {
            for tok in tokens {
                tok_send.send(tok).unwrap();
            }

            // manually drop the sender so that the receiver actually finishes iterating
            drop(tok_send);
        });

        // then, spawn num_threads worker threads for processing
        for _ in 0..num_threads {
            // clone the receiver and sender necessary for the worker
            let (r, s) = (tok_recv.clone(), val_send.clone());

            // just give the thread a closure that calls the worker function
            sc.spawn(move |_| worker(r, s));
        }

        // manaully drop this sender too
        drop(val_send);

        // spawn a bufwriter since we'll be writing TONS of single-lines
        let mut writer = BufWriter::new(output_file);
        // iterate over the received tokens
        for val in val_recv.iter() {
            // if the received value was validated successfully
            if val.is_some() {
                num_validated += 1;
                writer
                    .write_fmt(format_args!("{}\n", val.unwrap()))
                    .unwrap();
            }
        }
    })
    .unwrap();

    println!(
        "out of [{}] tokens, found [{}] to be valid",
        total_tokens, num_validated
    );
    println!("wrote valid tokens to [{}]", args[2]);
    Ok(())
}

fn worker(r: Receiver<Token>, s: Sender<Validated>) -> Result<()> {
    for tok in r.iter() {
        if validate(&tok) {
            s.send(Some(tok))?;
        } else {
            s.send(None)?;
        }
    }

    Ok(())
}

fn validate(tok: &str) -> bool {
    use reqwest::header::AUTHORIZATION;
    use reqwest::header::CONTENT_TYPE;
    use reqwest::StatusCode;

    const URL: &str = "https://discordapp.com/api/v6/users/@me/library";
    lazy_static! {
      // use lazy_static to keep the same client for the validator function
      static ref CLIENT: Client = Client::new();
    }

    // generate the headers for hte request
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(AUTHORIZATION, tok.parse().unwrap());
    // we unwrap the value here
    // that's fine, this will only fail in *rare* circumstances
    let resp = CLIENT.get(URL).headers(headers).send().unwrap();

    // if disord gives us an OK then the token is valid
    let status = resp.status();
    match status {
        StatusCode::OK => true,
        StatusCode::TOO_MANY_REQUESTS => {
            let wait = resp.headers().get("Retry-After")
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            
            println!("rate limited, waiting [{}s]", wait);
            thread::sleep(Duration::from_secs(wait));
            validate(tok)
        }
        _ => false
    }
}

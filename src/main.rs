#[macro_use]
extern crate anyhow;
extern crate crossbeam;
extern crate num_cpus;
extern crate reqwest;
#[macro_use]
extern crate log;
extern crate simplelog;

mod validate;

use anyhow::Result;
use clap::{App, Arg};
use crossbeam::channel::bounded;
use crossbeam::channel::{Receiver, Sender};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use validate::Validator;

type Token = String;
type Validated = Option<String>;

fn main() -> Result<()> {
    let level = if let Some(val) = env::vars().find(|i| i.0 == "TOKVAL_LOG") {
        let mut val = val.1;
        val.make_ascii_lowercase();

        match val.as_str() {
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "info" => log::LevelFilter::Info,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _ => {
                warn!("unknown log level: {}", val);
                log::LevelFilter::Info
            }
        }
    } else {
        log::LevelFilter::Info
    };

    let matches = App::new("tokval")
        .version("2.2.2")
        .author("by 9th")
        .about("high-speed discord token validator\nsee https://github.com/9thlast/tokval for documentation")
        .arg(
            Arg::with_name("input_file")
                .short("i")
                .long("input")
                .help("file containing a line-separated list of tokens")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("output_file")
                .short("o")
                .long("output")
                .help("file to write all valid tokens to")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("proxies")
                .short("p")
                .long("proxies")
                .value_name("proxyfile")
                .help("file containing a line-separated list of proxies")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("jobs")
                .short("j")
                .long("jobs")
                .value_name("# jobs")
                .help("number of threads to spawn (defaults to # cpus available)")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("enables verbose logging")
        )
        .arg(
            Arg::with_name("log")
                .short("l")
                .long("log")
                .value_name("log file")
                .help("file to output logs to")
                .takes_value(true)
        )
        .get_matches();

    // assign level to trace if the user passed the verbose option
    // otherwise just leave it as whatever we computed before
    let level = if matches.is_present("verbose") {
        log::LevelFilter::Debug
    } else {
        level
    };

    let log_cfg = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("tokval")
        .build();
    
    // if the user supplied the log option, then we want to init the logger
    // to write to the supplied log file
    if matches.is_present("log") {
        let logfp = matches.value_of("log").unwrap();
        let logfile = OpenOptions::new()
            .write(true)
            .read(false)
            .create(true)
            .open(logfp)
            .expect("could not open supplied log file for writing!");
        
        simplelog::WriteLogger::init(
            level,
            log_cfg,
            logfile
        )?;
    } else { // otherwise, we just need to initialize a TermLogger for stderr
        simplelog::TermLogger::init(
            level,
            log_cfg,
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto
        )?;
    }

    let input_path = matches.value_of("input_file").unwrap_or("stdin");
    let input_file: Option<File> = if matches.is_present("input_file") {
        // open input file and ensure it's good
        let input_file = OpenOptions::new().read(true).write(false).open(input_path);
        let input_file = match input_file {
            Ok(f) => f,
            Err(e) => {
                error!("error opening input file [{}]: {}", input_path, e);
                return Err(anyhow!(e));
            }
        };

        Some(input_file)
    } else {
        None
    };

    let output_path = matches.value_of("output_file").unwrap_or("stdout");
    let output_file: Option<File> = if matches.is_present("output_file") {
        // open output file and ensure it's good
        let output_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(output_path);
        let output_file = match output_file {
            Ok(f) => f,
            Err(e) => {
                error!("error opening output file: [{}]: {}", output_path, e);
                return Err(anyhow!(e));
            }
        };

        Some(output_file)
    } else {
        None
    };

    let validator = if matches.is_present("proxies") {
        let proxy_path = matches.value_of("proxies").unwrap();
        let proxy_file = OpenOptions::new().read(true).write(false).open(proxy_path);
        let proxy_file = match proxy_file {
            Ok(f) => f,
            Err(e) => {
                error!("error opening proxy file [{}]: {}", proxy_path, e);
                return Err(anyhow!(e));
            }
        };

        let mut proxies: Vec<String> = Vec::new();
        for line in BufReader::new(proxy_file).lines() {
            let line = line?;
            if !line.trim().is_empty() {
                proxies.push(line);
            }
        }
        info!("initialized [{}] proxied clients", proxies.len());

        Validator::from(proxies)?
    } else {
        Validator::new()
    };

    // spawn as many threads as the cpu has
    let num_threads = if matches.is_present("jobs") {
        match matches.value_of("jobs").unwrap().parse::<usize>() {
            Ok(v) => v,
            Err(e) => {
                error!("unable to parse number of jobs (must specify an integer)");
                return Err(anyhow!(e));
            }
        }
    } else {
        // a little bit of testing showed that, for each cpu the host machine has, we can support about 10 checker threads
        // so simply multiply the number of cpus by 10 to get the # threads
        num_cpus::get() * 10
    };

    // create the initial senders and receivers
    let (tok_send, tok_recv) = bounded::<Token>(1);
    let (val_send, val_recv) = bounded::<Validated>(1);

    // load all tokens in from the input file
    let mut tokens: Vec<String> = Vec::new();
    // to prevent having two entirely separate branches for reading input
    // we can box a BufRead instance to achieve the polymorphism we want so we can call .lines on it

    // this stdin object needs to be in the main scope, since the lock on it requires it to live long enough
    let stdin = io::stdin();
    let reader: Box<dyn BufRead> = match input_file {
        Some(f) => Box::new(BufReader::new(f)),
        None => Box::new(stdin.lock()),
    };
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            tokens.push(line.trim().to_string());
        }
    }

    info!("read [{}] tokens", tokens.len());
    info!("spawning [{}] worker threads", num_threads);

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
        for i in 0..num_threads {
            // clone the receiver and sender necessary for the worker
            let (r, s) = (tok_recv.clone(), val_send.clone());

            // just give the thread a closure that calls the worker function
            let mut cloned = validator.clone();
            cloned.set_client_offset(num_threads * i);
            sc.spawn(move |_| worker(cloned, r, s));
        }

        // manaully drop this sender too
        drop(val_send);

        // spawn a bufwriter since we'll be writing TONS of single-lines
        let output_file: Box<dyn Write> = match output_file {
            Some(f) => Box::new(BufWriter::new(f)),
            None => Box::new(BufWriter::new(io::stdout())),
        };
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

    info!(
        "out of [{}] tokens, found [{}] to be valid",
        total_tokens, num_validated
    );
    info!("wrote valid tokens to [{}]", output_path);
    Ok(())
}

fn worker(mut v: Validator, r: Receiver<Token>, s: Sender<Validated>) -> Result<()> {
    for tok in r.iter() {
        if v.validate(&tok) {
            s.send(Some(tok))?;
        } else {
            s.send(None)?;
        }
    }

    Ok(())
}

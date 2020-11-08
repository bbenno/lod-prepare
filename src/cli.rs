//! Command Line Interface module

#![warn(missing_docs)]

use std::{env, process};

pub fn get_args() -> Vec<String> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    match args.len() {
        2 => println!("Open database {}", &args[1]),
        _ => {
            eprintln!("Usage: {} DB_PATH\nDB_PATH is the name of an SQLite database.", &args[0]);
            process::exit(exitcode::USAGE);
        },
    }
    args
}
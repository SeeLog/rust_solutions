use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

use clap::{Arg, ArgAction, Command};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    number_lines: bool,
    number_nonblank_lines: bool,
}

pub fn run(config: Config) -> MyResult<()> {
    let files = config.files.clone();
    for filename in files {
        match open(&filename) {
            Err(e) => eprintln!("Failed to open {}: {}", filename, e),
            Ok(reader) => print_lines(reader, &config),
        }
    }
    Ok(())
}

pub fn get_args() -> MyResult<Config> {
    let matches = Command::new("catr")
        .version("0.1.0")
        .author("SeeLog <seelog693@gmail.com>")
        .about("Rust cat")
        .arg(
            Arg::new("files")
                .value_name("FILE")
                .help("Input file(s)")
                .num_args(1..)
                .default_value("-"),
        )
        .arg(
            Arg::new("number")
                .short('n')
                .long("number")
                .action(ArgAction::SetTrue)
                .help("output line numbers")
                .conflicts_with("number_nonblank"),
        )
        .arg(
            Arg::new("number_nonblank")
                .short('b')
                .long("number-nonblank")
                .action(ArgAction::SetTrue)
                .help("output non-blank line numbers"),
        )
        .get_matches();

    Ok(Config {
        files: matches
            .get_many::<String>("files")
            .expect("No files")
            .map(|x| x.to_string())
            .collect::<Vec<String>>(),
        number_lines: matches.get_flag("number"),
        number_nonblank_lines: matches.get_flag("number_nonblank"),
    })
}

fn print_lines(reader: Box<dyn BufRead>, config: &Config) {
    let mut line_number = 0;
    for line in reader.lines() {
        match line {
            Ok(line) => {
                if config.number_nonblank_lines {
                    if !line.is_empty() {
                        line_number += 1;
                        print!("{:6}\t", line_number);
                    }
                    println!("{}", line);
                } else if config.number_lines {
                    line_number += 1;
                    println!("{:6}\t{}", line_number, line);
                } else {
                    line_number += 1;
                    println!("{}", line);
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

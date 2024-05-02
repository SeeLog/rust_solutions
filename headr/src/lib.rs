use clap::{Arg, Command};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: usize,
    bytes: Option<usize>,
}

pub fn run(config: Config) -> MyResult<()> {
    for (i, filename) in config.files.iter().enumerate() {
        match open(filename) {
            Err(e) => eprintln!("headr: {}: {}", filename, e),
            Ok(stream) => {
                if config.files.len() > 1 {
                    if i > 0 {
                        println!();
                    }
                    println!("==> {} <==", filename);
                }
                if let Some(bytes) = config.bytes {
                    show_bytes(stream, bytes)?;
                } else {
                    show_lines(stream, config.lines)?;
                }
            }
        }
    }
    Ok(())
}

fn show_lines(mut reader: Box<dyn BufRead>, lines: usize) -> MyResult<()> {
    for _ in 0..lines {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line);
        if bytes? == 0 {
            break;
        }

        print!("{}", line);
    }
    Ok(())
}

fn show_bytes(mut reader: Box<dyn BufRead>, bytes: usize) -> MyResult<()> {
    let mut buf = vec![0; bytes];
    let result = reader.read_exact(buf.as_mut_slice());
    if let Err(e) = result {
        eprintln!("headr: error reading '{}': {}", "stdin", e);
        return Ok(());
    }
    print!("{}", String::from_utf8_lossy(&buf));

    Ok(())
}

pub fn get_args() -> MyResult<Config> {
    let matches = Command::new("headr")
        .version("0.1.0")
        .author("SeeLog")
        .arg(
            Arg::new("files")
                .value_name("FILES")
                .help("Input files")
                .num_args(1..)
                .default_value("-"),
        )
        .arg(
            Arg::new("lines")
                .value_name("LINES")
                .short('n')
                .long("lines")
                .help("Number of lines")
                .num_args(1)
                // .value_parser(clap::value_parser!(usize))
                .default_value("10"),
        )
        .arg(
            Arg::new("bytes")
                .value_name("BYTES")
                .short('c')
                .long("bytes")
                .help("Number of bytes")
                .num_args(1)
                // .value_parser(clap::value_parser!(usize))
                .conflicts_with("lines"),
        )
        .get_matches();

    let lines = matches
        .get_one::<String>("lines")
        .map(|v| parse_positive_int(v))
        .transpose()
        .map_err(|e| {
            format!(
                "error: invalid value '{}' for '--lines <LINES>': invalid digit found in string",
                e
            )
        })?;
    let bytes = matches
        .get_one::<String>("bytes")
        .map(|v| parse_positive_int(v))
        .transpose()
        .map_err(|e| {
            format!(
                "error: invalid value '{}' for '--bytes <BYTES>': invalid digit found in string",
                e
            )
        })?;

    Ok(Config {
        files: matches
            .get_many::<String>("files")
            .expect("No files")
            .cloned()
            .collect(),
        // lines: matches.get_one::<usize>("lines").expect("No lines").clone(),
        // bytes: matches.get_one::<usize>("bytes").copied(),
        lines: lines.unwrap_or(10),
        bytes,
    })
}

pub fn parse_positive_int(val: &str) -> MyResult<usize> {
    match val.parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(val.into()),
    }
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

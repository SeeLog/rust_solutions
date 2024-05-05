use clap::{Arg, ArgAction, Command};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    in_file: String,
    out_file: Option<String>,
    count: bool,
}

pub fn run(config: Config) -> MyResult<()> {
    let mut file = open(&config.in_file).map_err(|e| format!("{}: {}", &config.in_file, e))?;
    let mut out: Box<dyn Write> = match &config.out_file {
        Some(out_filename) => Box::new(File::create(out_filename)?),
        _ => Box::new(io::stdout()),
    };
    let mut line = String::new();
    let mut before = String::new();
    let mut count: usize = 0;

    let mut write = |count: usize, text: &str| -> MyResult<()> {
        if count > 0 {
            if config.count {
                write!(out, "{:4} {}", count, text)?;
            } else {
                write!(out, "{}", text)?;
            }
        }

        Ok(())
    };

    loop {
        let bytes = file.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        // 違うやつが来た
        if line.trim_end() != before.trim_end() {
            write(count, &before)?;
            before = line.clone();
            count = 0;
        }
        count += 1;
        line.clear();
    }

    write(count, &before)?;

    Ok(())
}

pub fn get_args() -> MyResult<Config> {
    let matches = Command::new("uniqr")
        .version("0.1.0")
        .author("SeeLog")
        .about("Rust uniq")
        .arg(
            Arg::new("in_file")
                .value_name("IN_FILE")
                .num_args(1)
                .default_value("-")
                .help("Input file"),
        )
        .arg(
            Arg::new("out_file")
                .value_name("OUT_FILE")
                .num_args(1)
                .help("Output file"),
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .action(ArgAction::SetTrue)
                .help("Show counts"),
        )
        .get_matches();

    let in_file = matches.get_one::<String>("in_file").unwrap().to_string();
    let out_file = matches.get_one::<String>("out_file").map(String::from);
    let count = matches.get_flag("count");

    return Ok(Config {
        in_file,
        out_file,
        count,
    });
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

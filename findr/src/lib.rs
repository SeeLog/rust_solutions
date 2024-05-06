use crate::EntryType::*;
use clap::{builder::PossibleValuesParser, Arg, ArgAction, Command};
use regex::Regex;
use std::error::Error;
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Eq, PartialEq)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Vec<Regex>,
    entry_types: Vec<EntryType>,
}

pub fn run(config: Config) -> MyResult<()> {
    for path in config.paths {
        for entry in WalkDir::new(path).into_iter().filter(|e| match e {
            Err(e) => {
                eprintln!("{}", e);
                false
            }
            Ok(e) => {
                (config.names.is_empty()
                    || config.names.iter().any(|regex| {
                        regex.is_match(
                            e.path()
                                .file_name()
                                .unwrap_or_default()
                                .to_str()
                                .unwrap_or_default(),
                        )
                    }))
                    && (config.entry_types.is_empty()
                        || config.entry_types.iter().any(|t| match t {
                            Dir => e.path().is_dir(),
                            File => e.path().is_file(),
                            Link => e.path().is_symlink(),
                        }))
            }
        }) {
            match entry {
                Err(e) => eprintln!("{}", e),
                Ok(entry) => println!("{}", entry.path().display()),
            }
        }
    }
    Ok(())
}

pub fn get_args() -> MyResult<Config> {
    let matches = Command::new("findr")
        .version("0.1.0")
        .author("SeeLog")
        .about("Rust find")
        .arg(
            Arg::new("paths")
                .value_name("PATHS")
                .num_args(1..)
                .default_value(".")
                .help("Search paths"),
        )
        .arg(
            Arg::new("names")
                .value_name("NAMES")
                .short('n')
                .long("name")
                .num_args(0..)
                .action(ArgAction::Append)
                .value_parser(|s: &str| Regex::new(s))
                .help("File name(s)"),
        )
        .arg(
            Arg::new("types")
                .value_name("TYPE")
                .short('t')
                .long("type")
                .num_args(0..)
                .action(ArgAction::Append)
                .value_parser(PossibleValuesParser::new(&["d", "f", "l"])),
        )
        .get_matches();

    let paths = matches
        .get_many::<String>("paths")
        .unwrap()
        .map(|s| s.to_string())
        .collect();
    let names = matches
        .get_many::<Regex>("names")
        .unwrap_or_default()
        .cloned()
        .collect();

    let entry_types = matches
        .get_many::<String>("types")
        .unwrap_or_default()
        .map(|s| match s.as_str() {
            "d" => Dir,
            "f" => File,
            "l" => Link,
            _ => unreachable!(),
        })
        .collect();

    Ok(Config {
        paths,
        names,
        entry_types,
    })
}

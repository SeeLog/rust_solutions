use clap::{Arg, ArgAction, Command};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
}

#[derive(Debug, PartialEq)]
pub struct FileInfo {
    num_lines: usize,
    num_words: usize,
    num_bytes: usize,
    num_chars: usize,
}

pub fn run(config: Config) -> MyResult<()> {
    let mut total_info = FileInfo {
        num_lines: 0,
        num_words: 0,
        num_bytes: 0,
        num_chars: 0,
    };
    for filename in &config.files {
        match open(filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => {
                let info = count(file);

                match info {
                    Err(e) => eprintln!("{}: {}", filename, e),
                    Ok(info) => {
                        print_info(&info, &config, filename);

                        total_info.num_lines += info.num_lines;
                        total_info.num_words += info.num_words;
                        total_info.num_bytes += info.num_bytes;
                        total_info.num_chars += info.num_chars;
                    }
                }
            }
        }
    }
    if config.files.len() > 1 {
        print_info(&total_info, &config, "total");
    }
    Ok(())
}

fn print_info(info: &FileInfo, config: &Config, filename: &str) {
    if config.lines {
        print!("{:8}", info.num_lines);
    }
    if config.words {
        print!("{:8}", info.num_words);
    }
    if config.bytes {
        print!("{:8}", info.num_bytes);
    }
    if config.chars {
        print!("{:8}", info.num_chars);
    }
    if filename == "-" {
        println!();
    } else {
        println!(" {}", filename);
    }
}

pub fn count(mut file: impl BufRead) -> MyResult<FileInfo> {
    let mut num_lines = 0;
    let mut num_words = 0;
    let mut num_bytes = 0;
    let mut num_chars = 0;

    let mut line = String::new();

    loop {
        line.clear();
        let result = file.read_line(&mut line);
        if result.is_err() || line.is_empty() {
            break;
        }

        num_lines += 1;
        num_words += line.split_whitespace().count();
        num_bytes += line.len();
        num_chars += line.chars().count();
    }

    Ok(FileInfo {
        num_lines,
        num_words,
        num_bytes,
        num_chars,
    })
}

pub fn get_args() -> MyResult<Config> {
    let matches = Command::new("wcr")
        .version("0.1.0")
        .author("SeeLog")
        .about("Rust wc")
        .arg(
            Arg::new("files")
                .value_name("FILES")
                .num_args(1..)
                .default_value("-")
                .help("Input file(s)"),
        )
        .arg(
            Arg::new("lines")
                .value_name("LINES")
                .short('l')
                .long("lines")
                .action(ArgAction::SetTrue)
                .help("Show line count"),
        )
        .arg(
            Arg::new("words")
                .value_name("WORDS")
                .short('w')
                .long("words")
                .action(ArgAction::SetTrue)
                .help("Show word count"),
        )
        .arg(
            Arg::new("bytes")
                .value_name("BYTES")
                .short('c')
                .long("bytes")
                .action(ArgAction::SetTrue)
                .help("Show byte count"),
        )
        .arg(
            Arg::new("chars")
                .value_name("CHARS")
                .short('m')
                .long("chars")
                .action(ArgAction::SetTrue)
                .conflicts_with("bytes")
                .help("Show character count"),
        )
        .get_matches();

    let files = matches
        .get_many::<String>("files")
        .unwrap()
        .map(|s| s.to_string())
        .collect();

    let mut lines = matches.get_flag("lines");
    let mut words = matches.get_flag("words");
    let mut bytes = matches.get_flag("bytes");
    let chars = matches.get_flag("chars");

    if [lines, words, bytes, chars].iter().all(|v| !v) {
        // 全部 false ならデフォルトをセット
        lines = true;
        words = true;
        bytes = true;
    }

    Ok(Config {
        files,
        lines,
        words,
        bytes,
        chars,
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

#[cfg(test)]
mod tests {
    use super::{count, FileInfo};

    use std::io::Cursor;

    #[test]
    fn test_count() {
        let text = "I don't want the world. I just want your half.\r\n";
        let info = count(Cursor::new(text));
        assert!(info.is_ok());
        let expected = FileInfo {
            num_lines: 1,
            num_words: 10,
            num_chars: 48,
            num_bytes: 48,
        };
        assert_eq!(info.unwrap(), expected);
    }
}

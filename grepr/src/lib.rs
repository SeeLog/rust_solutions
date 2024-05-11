use clap::{Arg, ArgAction, Command};
use regex::{Regex, RegexBuilder};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    pattern: Regex,
    files: Vec<String>,
    recursive: bool,
    count: bool,
    invert_match: bool,
}

pub fn run(config: Config) -> MyResult<()> {
    let entries = find_files(&config.files, config.recursive);
    for entry in &entries {
        match entry {
            Err(e) => eprintln!("{}", e),
            Ok(filename) => match open(&filename) {
                Err(e) => eprintln!("{}: {}", filename, e),
                Ok(file) => {
                    let matches = find_lines(file, &config.pattern, config.invert_match);
                    if entries.len() > 1 {
                        print_match(&config, matches?, filename, true);
                    } else {
                        print_match(&config, matches?, filename, false);
                    }
                }
            },
        }
    }

    Ok(())
}

fn print_match(config: &Config, matches: Vec<String>, filename: &str, show_filename: bool) {
    if config.count {
        if show_filename {
            print!("{}:", filename);
        }
        println!("{}", matches.len());
    } else {
        matches.iter().for_each(|m| {
            if show_filename {
                print!("{}:", filename);
            }
            print!("{}", m)
        });
    }
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = Command::new("grepr")
        .version("0.1.0")
        .author("SeeLog")
        .about("Rust grep")
        .arg(
            Arg::new("pattern")
                .value_name("PATTERN")
                .help("Search pattern")
                .required(true)
                .num_args(1),
        )
        .arg(
            Arg::new("files")
                .value_name("FILE")
                .help("Input file(s)")
                .num_args(1..)
                .default_value("-"),
        )
        .arg(
            Arg::new("recursive")
                .value_name("RECURSIVE")
                .short('r')
                .long("recursive")
                .action(ArgAction::SetTrue)
                .help("Recursive search"),
        )
        .arg(
            Arg::new("count")
                .value_name("COUNT")
                .short('c')
                .long("count")
                .action(ArgAction::SetTrue)
                .help("Count occurrences"),
        )
        .arg(
            Arg::new("invert_match")
                .value_name("INVERT")
                .short('v')
                .long("invert-match")
                .action(ArgAction::SetTrue)
                .help("Invert match"),
        )
        .arg(
            Arg::new("insensitive")
                .value_name("INSENSITIVE")
                .short('i')
                .long("insensitive")
                .action(ArgAction::SetTrue)
                .help("Case insensitive"),
        )
        .get_matches();

    let insensitive = matches.get_flag("insensitive");
    let pattern_string = matches.get_one::<String>("pattern").unwrap();
    let pattern = RegexBuilder::new(pattern_string)
        .case_insensitive(insensitive)
        .build()
        .map_err(|_| format!("Invalid pattern \"{}\"", pattern_string))?;
    let files = matches
        .get_many::<String>("files")
        .unwrap()
        .map(|s| s.to_string())
        .collect();
    let recursive = matches.get_flag("recursive");
    let count = matches.get_flag("count");
    let invert_match = matches.get_flag("invert_match");

    Ok(Config {
        pattern,
        files,
        recursive,
        count,
        invert_match,
    })
}

fn find_lines<T: BufRead>(
    mut file: T,
    pattern: &Regex,
    invert_match: bool,
) -> MyResult<Vec<String>> {
    let mut matches = vec![];
    let mut line = String::new();

    loop {
        let bytes = file.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        if pattern.is_match(&line) != invert_match {
            matches.push(line.clone());
        }
        line.clear();
    }

    Ok(matches)
}

fn find_files(paths: &[String], recursive: bool) -> Vec<MyResult<String>> {
    let mut files: Vec<MyResult<String>> = vec![];
    for path in paths {
        let path = path.replace("\\", "/");
        if path == "-" {
            files.push(Ok(path));
            continue;
        }
        let metadata = std::fs::metadata(&path);
        if metadata.is_err() {
            files.push(Err(format!("{}: {}", path, metadata.err().unwrap(),).into()));
            continue;
        }

        let metadata = metadata.unwrap();
        if metadata.is_file() {
            files.push(Ok(path));
        } else if metadata.is_dir() && recursive {
            let ex_files = walkdir::WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| match e {
                    Ok(e) => {
                        if e.path().is_file() {
                            Some(e.path().display().to_string())
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        files.push(Err(format!("{}: {}", path, e).into()));
                        None
                    }
                })
                .collect::<Vec<String>>();
            files.extend(ex_files.into_iter().map(|s| Ok(s)));
        } else if metadata.is_dir() {
            files.push(Err(format!("{} is a directory", path).into()));
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::find_lines;

    use super::find_files;
    use rand::{distributions::Alphanumeric, thread_rng, Rng};
    use regex::{Regex, RegexBuilder};

    #[test]
    fn test_find_files() {
        // 1個のファイルが探せる
        let files = find_files(&["./tests/inputs/fox.txt".to_string()], false);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].as_ref().unwrap(), "./tests/inputs/fox.txt");

        // recursive なしの場合、ディレクトリはエラー
        let files = find_files(&["./tests/inputs".to_string()], false);
        assert_eq!(files.len(), 1);
        if let Err(e) = &files[0] {
            assert_eq!(e.to_string(), "./tests/inputs is a directory");
        }

        // recursive ありの場合、ディレクトリ内を再帰的に探せる
        let res = find_files(&["./tests/inputs".to_string()], true);
        let mut files: Vec<String> = res
            .iter()
            .map(|r| r.as_ref().unwrap().replace("\\", "/"))
            .collect();
        files.sort();
        println!("{:?}", files);
        assert_eq!(files.len(), 4);
        assert_eq!(
            files,
            vec![
                "./tests/inputs/bustle.txt",
                "./tests/inputs/empty.txt",
                "./tests/inputs/fox.txt",
                "./tests/inputs/nobody.txt",
            ]
        );

        // 存在しないファイルをランダム生成してテスト
        let bad: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        let files = find_files(&[bad], false);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_err());
    }

    #[test]
    fn test_find_lines() {
        let text = b"Lorem\nIpsum\r\nDOLOR";

        // or は Lorem にマッチ
        let rel = Regex::new("or").unwrap();
        let matches = find_lines(Cursor::new(&&text), &rel, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);

        // invert_match ありの場合、Lorem 以外にマッチ
        let matches = find_lines(Cursor::new(&&text), &rel, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // 大文字・小文字を区別しない
        let re2 = RegexBuilder::new("or")
            .case_insensitive(true)
            .build()
            .unwrap();

        // Lorem と DOLOR にマッチ
        let matches = find_lines(Cursor::new(&&text), &re2, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // invert_match ありの場合、Lorem と DOLOR 以外にマッチ
        let matches = find_lines(Cursor::new(&&text), &re2, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
    }
}

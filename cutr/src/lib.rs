use crate::Extract::*;
use clap::{Arg, Command};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use regex::Regex;
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
    ops::Range,
    vec,
};

type MyResult<T> = Result<T, Box<dyn Error>>;
type PositionList = Vec<Range<usize>>;

#[derive(Debug)]
pub enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    delimiter: u8,
    extract: Extract,
}

pub fn run(config: Config) -> MyResult<()> {
    for filename in &config.files {
        match open(filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(file) => match &config.extract {
                Fields(field_pos) => {
                    let mut reader = ReaderBuilder::new()
                        .delimiter(config.delimiter)
                        .has_headers(false)
                        .from_reader(file);
                    let mut writer = WriterBuilder::new()
                        .delimiter(config.delimiter)
                        .from_writer(io::stdout());

                    for record in reader.records() {
                        let record = record?;
                        writer.write_record(extract_fields(&record, field_pos))?;
                    }
                }
                Bytes(byte_pos) => {
                    for line in file.lines() {
                        println!("{}", extract_bytes(&line?, byte_pos));
                    }
                }
                Chars(char_pos) => {
                    for line in file.lines() {
                        println!("{}", extract_chars(&line?, char_pos));
                    }
                }
            },
        }
    }
    Ok(())
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let chars: Vec<_> = line.chars().collect();
    let mut result = String::new();

    for range in char_pos.iter().cloned() {
        for i in range {
            if let Some(val) = chars.get(i) {
                result.push(*val);
            }
        }
    }

    result
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let bytes = line.as_bytes();
    let result: Vec<u8> = byte_pos
        .iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| bytes.get(i)))
        .cloned()
        .collect();

    String::from_utf8_lossy(&result).to_string()
}

// fn extract_fields<'a>(record: &'a StringRecord, field_pos: &[Range<usize>]) -> Vec<&'a str> {
//     field_pos
//         .iter()
//         .cloned()
//         .flat_map(|range| range.filter_map(|i| record.get(i)))
//         .collect()
// }

fn extract_fields(record: &StringRecord, field_pos: &[Range<usize>]) -> Vec<String> {
    field_pos
        .iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| record.get(i)))
        .map(String::from)
        .collect()
}

pub fn get_args() -> MyResult<Config> {
    let matches = Command::new("cutr")
        .version("0.1.0")
        .author("SeeLog")
        .about("Rust cut")
        .arg(
            Arg::new("files")
                .value_name("FILE")
                .num_args(1..)
                .default_value("-")
                .help("Input file(s)"),
        )
        .arg(
            Arg::new("bytes")
                .value_name("BYTES")
                .short('b')
                .long("bytes")
                .conflicts_with_all(vec!["chars", "fields"])
                .help("Selected bytes"),
        )
        .arg(
            Arg::new("chars")
                .value_name("CHARS")
                .short('c')
                .long("characters")
                .conflicts_with_all(vec!["bytes", "fields"])
                .help("Selected characters"),
        )
        .arg(
            Arg::new("delimiter")
                .value_name("DELIMITER")
                .short('d')
                .long("delimiter")
                .default_value("\t")
                .help("Field delimiter"),
        )
        .arg(
            Arg::new("fields")
                .value_name("FIELDS")
                .short('f')
                .long("fields")
                .conflicts_with_all(vec!["bytes", "chars"])
                .help("Selected fields"),
        )
        .get_matches();

    let files = matches
        .get_many::<String>("files")
        .unwrap()
        .cloned()
        .collect();
    let delimiter = matches.get_one::<String>("delimiter").unwrap();
    let delimiter_bytes = delimiter.as_bytes();
    if delimiter_bytes.len() != 1 {
        return Err(format!("--delim \"{}\" must be a single byte", delimiter).into());
    }

    let extract = if let Some(range) = matches.get_one::<String>("bytes") {
        Bytes(parse_pos(range)?)
    } else if let Some(range) = matches.get_one::<String>("chars") {
        Chars(parse_pos(range)?)
    } else if let Some(range) = matches.get_one::<String>("fields") {
        Fields(parse_pos(range)?)
    } else {
        return Err("the following required arguments were not provided:\n  \
        <--fields <FIELDS>|--bytes <BYTES>|--chars <CHARS>>"
            .into());
    };

    Ok(Config {
        files,
        delimiter: *delimiter_bytes.first().unwrap(),
        extract,
    })
}

fn parse_pos(range: &str) -> MyResult<PositionList> {
    let mut pos = Vec::new();
    let compose_err_msg = |s: &str| format!("illegal list value: {:?}", s);

    let convert_to_range_usize = |s: &str| -> MyResult<usize> {
        match s.parse::<usize>() {
            Err(_) => Err(compose_err_msg(s).into()),
            Ok(v) => {
                if v <= 0 {
                    Err(compose_err_msg(s).into())
                } else {
                    Ok(v - 1)
                }
            }
        }
    };

    for item in range.split(',') {
        if item.contains('+') {
            return Err(compose_err_msg(item).into());
        }

        match convert_to_range_usize(item) {
            Ok(item) => {
                pos.push(item..item + 1);
            }
            Err(_) => {
                let re = Regex::new(r"(\d+)-(\d+)").unwrap();
                if !re.is_match(item) {
                    return Err(compose_err_msg(item).into());
                }

                let cap = re.captures(item).unwrap();
                let start = convert_to_range_usize(cap.get(1).unwrap().as_str())?;
                let end = convert_to_range_usize(cap.get(2).unwrap().as_str())?;

                if start >= end {
                    return Err(format!(
                        "First number in range ({}) must be lower than second number ({})",
                        start + 1,
                        end + 1
                    )
                    .into());
                }

                pos.push(start..end + 1);
            }
        }
    }

    Ok(pos)
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

#[cfg(test)]
mod unit_tests {
    use csv::StringRecord;

    use super::parse_pos;
    use crate::{extract_bytes, extract_chars, extract_fields};

    #[test]
    fn test_parse_pos() {
        // 空文字はエラー
        assert!(parse_pos("").is_err());
        // ゼロはエラー
        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"");

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"");

        // 数字の前に + が付く場合はエラー
        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1\"");

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1-2\"");

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-+2\"");

        // 数字以外はエラー
        let res = parse_pos("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"");

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"");

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-a\"");

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a-1\"");

        // エラーになる範囲
        let res = parse_pos("-");
        assert!(res.is_err());

        let res = parse_pos(",");
        assert!(res.is_err());

        let res = parse_pos("1,");
        assert!(res.is_err());

        let res = parse_pos("1-");
        assert!(res.is_err());

        let res = parse_pos("1-1-1");
        assert!(res.is_err());

        let res = parse_pos("1-1-a");
        assert!(res.is_err());

        // 最初の数字は2番目より小さい必要がある
        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        // 以下は OK
        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("あbc", &[0..1]), "あ".to_string());
        assert_eq!(extract_chars("あbc", &[0..1, 2..3]), "あc".to_string());
        assert_eq!(extract_chars("あbc", &[0..3]), "あbc".to_string());
        assert_eq!(extract_chars("あbc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(
            extract_chars("あbc", &[0..1, 1..2, 4..5]),
            "あb".to_string()
        );
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("あbc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("あbc", &[0..3]), "あ".to_string());
        assert_eq!(extract_bytes("あbc", &[0..4]), "あb".to_string());
        assert_eq!(extract_bytes("あbc", &[0..5]), "あbc".to_string());
        assert_eq!(extract_bytes("あbc", &[4..5, 3..4]), "cb".to_string());
        assert_eq!(extract_bytes("あbc", &[0..3, 6..7]), "あ".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(extract_fields(&rec, &[0..1, 2..3]), &["Captain", "12345"]);
    }
}

use crate::Extract::*;
use clap::{Arg, Command};
use regex::Regex;
use std::{error::Error, ops::Range, vec};

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
    println!("{:?}", config);
    Ok(())
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

#[cfg(test)]
mod unit_tests {
    use super::parse_pos;

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
}

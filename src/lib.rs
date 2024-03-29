use std::fs::File;
use std::io::{BufRead, BufReader};
use std::num::NonZeroUsize;
use std::ops::Range;

use clap::{Args, Parser};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use regex::Regex;

pub type MyResult<T> = Result<T, Box<dyn std::error::Error>>;
type PositionList = Vec<Range<usize>>;

fn parse_index(input: &str) -> Result<usize, String> {
    let value_error = || format!("illegal list value: \"{}\"", input);

    input
        .starts_with('+')
        .then(|| Err(value_error()))
        .unwrap_or_else(|| {
            input.parse::<NonZeroUsize>()
                .map(|n| usize::from(n) - 1)
                .map_err(|_| value_error())
        })
}

fn parse_pos(range: &str) -> Result<PositionList, String> {
    let range_re = Regex::new(r"^(\d+)-(\d+)$").unwrap();

    range.split(',')
        .into_iter()
        .map(|val| {
            parse_index(val)
                .map(|n| n..n + 1)
                .or_else(|e| {
                    range_re.captures(val).ok_or(e).and_then(|captures| {
                        let n1 = parse_index(&captures[1])?;
                        let n2 = parse_index(&captures[2])?;
                        if n1 >= n2 {
                            return Err(format!("First number in range ({}) must be lower than second number ({})", n1 + 1, n2 + 1));
                        }

                        Ok(n1..n2 + 1)
                    })
                })
        }).collect::<Result<_, _>>()
        .map_err(|e| e.into())
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
struct Extract {
    #[arg(short, long, help = "Selected fields", value_parser = parse_pos)]
    fields: Option<PositionList>,
    #[arg(short, long, help = "Selected bytes", value_parser = parse_pos)]
    bytes: Option<PositionList>,
    #[arg(short, long, help = "Selected characters", value_parser = parse_pos)]
    chars: Option<PositionList>,
}

#[derive(Parser, Debug)]
#[command(name = "cutr")]
#[command(version = "0.1.0")]
#[command(about = "Rust cut")]
#[command(author = "Radish-Miyazaki <y.hidaka.kobe@gmail.com>")]
pub struct Cli {
    #[command(flatten)]
    extract: Extract,
    #[arg(value_name = "FILE", help = "Input file(s)", default_value = "-")]
    files: Vec<String>,
    #[arg(short, long = "delim", help = "Field delimiter", default_value = "\t")]
    delimiter: char,
}

pub fn get_args() -> MyResult<Cli> {
    Ok(Cli::parse())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(std::io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let mut str = String::new();
    for pos in char_pos {
        line.chars()
            .skip(pos.start)
            .take(pos.end - pos.start)
            .for_each(|c| str.push(c));
    }

    str
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let mut bytes: Vec<u8> = vec![];
    for pos in byte_pos {
        line.bytes()
            .skip(pos.start)
            .take(pos.end - pos.start)
            .for_each(|b| bytes.push(b));
    }

    String::from_utf8_lossy(&bytes).to_string()
}

fn extract_fields(
    record: &StringRecord,
    field_pos: &[Range<usize>],
) -> Vec<String> {
    let mut fields: Vec<String> = vec![];

    for pos in field_pos {
        record.iter()
            .skip(pos.start)
            .take(pos.end - pos.start)
            .for_each(|s| fields.push(s.to_string()));
    }

    fields
}

pub fn run(cli: Cli) -> MyResult<()> {
    for filename in &cli.files {
        match open(filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(f) => {
                if let Some(ref position_list) = cli.extract.chars {
                    for line in f.lines() {
                        let line = line?;
                        println!("{}", extract_chars(&line, &position_list));
                    }
                } else if let Some(ref position_list) = cli.extract.bytes {
                    for line in f.lines() {
                        let line = line?;
                        println!("{}", extract_bytes(&line, &position_list));
                    }
                } else if let Some(ref position_list) = cli.extract.fields {
                    let mut rdr = ReaderBuilder::new()
                        .has_headers(false)
                        .delimiter(cli.delimiter as u8)
                        .from_reader(f);

                    let mut wtr = WriterBuilder::new()
                        .delimiter(cli.delimiter as u8)
                        .from_writer(std::io::stdout());

                    for record in rdr.records() {
                        let record = record?;
                        let fields = extract_fields(&record, &position_list);
                        wtr.write_record(fields.iter())?;
                    }
                } else {
                    unimplemented!()
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_parse_pos() {
        assert!(parse_pos("").is_err());

        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"");

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"",);

        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"+1\"",
        );

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"+1-2\"",
        );

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"1-+2\"",
        );

        let res = parse_pos("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"1-a\"",
        );

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "illegal list value: \"a-1\"",
        );

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
        assert_eq!(extract_chars("ábc", &[0..1]), "á".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 2..3]), "ác".to_string());
        assert_eq!(extract_chars("ábc", &[0..3]), "ábc".to_string());
        assert_eq!(extract_chars("ábc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(
            extract_chars("ábc", &[0..1, 1..2, 4..5]),
            "áb".to_string()
        );
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("ábc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2]), "á".to_string());
        assert_eq!(extract_bytes("ábc", &[0..3]), "áb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..4]), "ábc".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2, 5..6]), "á".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(
            extract_fields(&rec, &[0..1, 2..3]),
            &["Captain", "12345"]
        );
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
    }
}

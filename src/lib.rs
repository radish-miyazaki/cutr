use std::num::NonZeroUsize;
use std::ops::Range;

use clap::{Args, Parser};
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
    fields: PositionList,
    #[arg(short, long, help = "Selected bytes", value_parser = parse_pos)]
    bytes: PositionList,
    #[arg(short, long, help = "Selected characters", value_parser = parse_pos)]
    chars: PositionList,
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

pub fn run(cli: Cli) -> MyResult<()> {
    println!("{:?}", cli);
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
}

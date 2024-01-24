use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    combinator::value,
    IResult,
};

#[derive(Clone, Debug, PartialEq)]
pub enum HoconValue {
    HoconString(String),
    HoconNumber(f64),
    HoconObject(HashMap<String, HoconValue>),
    HoconArray(Vec<HoconValue>),
    HoconBoolean(bool),
    HoconNull,
}

fn null<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
    let (input, _) = tag("null")(input)?;
    Ok((input, HoconValue::HoconNull))
}

fn boolean<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
    let parse_true = value(HoconValue::HoconBoolean(true), tag("true"));
    let parse_false = value(HoconValue::HoconBoolean(false), tag("false"));
    alt((parse_true, parse_false))(input)
}

fn whitespace<'a>(input: &'a str) -> IResult<&'a str, ()> {
    let (input, _) = take_while(|c: char| {
        c.is_whitespace()
            || c == '\t'
            || c == '\n'
            || c == '\u{000B}'
            || c == '\u{000C}'
            || c == '\r'
            || c == '\u{001C}'
            || c == '\u{001D}'
            || c == '\u{001E}'
            || c == '\u{001F}'
    })(input)?;
    Ok((input, ()))
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::parser::HoconValue;

    #[test]
    fn test_null() {
        assert_eq!(null("null"), Ok(("", HoconValue::HoconNull)));
    }

    #[test]
    fn test_boolean_true() {
        assert_eq!(boolean("true"), Ok(("", HoconValue::HoconBoolean(true))));
    }

    #[test]
    fn test_boolean_false() {
        assert_eq!(boolean("false"), Ok(("", HoconValue::HoconBoolean(false))));
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(whitespace("     test"), Ok(("test", ())));
    }
}

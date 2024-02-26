use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric0, alphanumeric1, char},
    combinator::{map, peek, value},
    multi::{many0, many1, many_m_n},
    number::complete::double,
    sequence::{delimited, tuple, Tuple},
    IResult,
};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct HoconObject<'a> {
    pub data: HashMap<String, HoconValue<'a>>
}

/// Represents a hocon value within the AST representation.
#[derive(Clone, Debug, PartialEq)]
pub enum HoconValue<'a> {
    HoconString(&'a str),
    HoconNumber(f64),
    HoconObject(HashMap<&'a str, HoconValue<'a>>),
    HoconArray(Vec<HoconValue<'a>>),
    HoconBoolean(bool),
    HoconNull,
}

/// Represents the various modes of failure while parsing or evaluating hocon files.
#[derive(Error, Debug, PartialEq)]
pub enum HoconError {
    // TODO Integrate better with nom error to get better parsing error docs
    #[error("Parse error")]
    ParseError,
}

/// Parses the given input as a Hocon document into a Hocon AST.
pub fn parse<'a>(input: &'a str) -> Result<HoconValue<'a>, HoconError> {
    let r = parse_object(input);
    match r {
        Ok((_, value)) => Ok(value),
        Err(_) => Err(HoconError::ParseError),
    }
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

fn string<'a>(input: &'a str) -> IResult<&'a str, &'a str> {
    fn parse_str<'a>(input: &'a str) -> IResult<&'a str, &str> {
        alphanumeric0(input)
    }

    alt((delimited(char('"'), parse_str, char('"')), alphanumeric1))(input)
}

fn number<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
    map(double, |num| HoconValue::HoconNumber(num))(input)
}

fn parse_value<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
    alt((
        null,
        boolean,
        number,
        map(string, |s| HoconValue::HoconString(s)),
        array,
        parse_object,
    ))(input)
}

fn next_element_whitespace<'a>(input: &'a str) -> IResult<&'a str, ()> {
    map(tuple((whitespace, many_m_n(0, 1, char(',')))), |_| ())(input)
}

fn key_value<'a>(input: &'a str) -> IResult<&'a str, (&'a str, HoconValue)> {
    fn separator<'a>(input: &'a str) -> IResult<&'a str, ()> {
        map(alt((char(':'), char('='), peek(char('{')))), |_| ())(input)
    }

    let (input, (_, path, _, _, _, value, _)) = (
        whitespace,
        string,
        whitespace,
        separator,
        whitespace,
        parse_value,
        next_element_whitespace,
    )
        .parse(input)?;
    Ok((input, (path, value)))
}

fn array<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
    fn array_element(input: &str) -> IResult<&str, HoconValue> {
        let (input, (_, value, _)) = (whitespace, parse_value, next_element_whitespace).parse(input)?;
        Ok((input, value))
    }

    delimited(
        char('['),
        map(many0(array_element), |elements| HoconValue::HoconArray(elements)),
        char(']'),
    )(input)
}

fn parse_object<'a>(input: &'a str) -> IResult<&'a str, HoconValue<'a>> {
    fn to_map<'a>(kvs: Vec<(&'a str, HoconValue<'a>)>) -> HoconValue<'a> {
        let mut map = HashMap::new();
        for (k, v) in kvs {
            map.insert(k, v);
        }
        HoconValue::HoconObject(map)
    }

    fn parse_inner0<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
        map(many0(key_value), to_map)(input)
    }

    fn parse_inner1<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
        map(many1(key_value), to_map)(input)
    }

    alt((delimited(char('{'), parse_inner0, char('}')), parse_inner1))(input)
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

    #[test]
    fn test_string() {
        assert_eq!(string("test"), Ok(("", "test")));
    }

    #[test]
    fn test_quoted_string() {
        assert_eq!(string("\"test\""), Ok(("", "test")));
    }

    #[test]
    fn test_key_value() {
        assert_eq!(
            key_value("test = true"),
            Ok(("", ("test", HoconValue::HoconBoolean(true))))
        );
    }

    #[test]
    fn test_number() {
        assert_eq!(number("42"), Ok(("", HoconValue::HoconNumber(42f64))));
    }

    #[test]
    fn test_array() {
        let expected_data = vec![
            HoconValue::HoconNumber(1f64),
            HoconValue::HoconNumber(2f64),
            HoconValue::HoconNumber(3f64),
        ];
        assert_eq!(array("[1,2,3]"), Ok(("", HoconValue::HoconArray(expected_data))));
    }

    #[test]
    fn test_array_trailing_comma() {
        let expected_data = vec![
            HoconValue::HoconNumber(1f64),
            HoconValue::HoconNumber(2f64),
            HoconValue::HoconNumber(3f64),
        ];
        assert_eq!(array("[1,2,3,]"), Ok(("", HoconValue::HoconArray(expected_data))));
    }

    #[test]
    fn parse_basic_json_object() {
        let content = r#"{ "hello": "world" }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello", HoconValue::HoconString("world"));
        assert_eq!(parse(&content), Ok(HoconValue::HoconObject(expected_map)));
    }

    #[test]
    fn parse_json_object_with_two_keys() {
        let content = r#"{ "hello": "world", "world": "hello" }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello", HoconValue::HoconString("world"));
        expected_map.insert("world", HoconValue::HoconString("hello"));
        assert_eq!(parse(&content), Ok(HoconValue::HoconObject(expected_map)));
    }

    #[test]
    fn parse_json_object_with_two_keys_multiline() {
        let content = r#"{
            "hello": "world",
            "world": "hello"
        }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello", HoconValue::HoconString("world"));
        expected_map.insert("world", HoconValue::HoconString("hello"));
        assert_eq!(parse(&content), Ok(HoconValue::HoconObject(expected_map)));
    }

    #[test]
    fn parse_hocon_object_with_two_keys() {
        let content = r#"
            hello: "world"
            world: "hello"
        }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello", HoconValue::HoconString("world"));
        expected_map.insert("world", HoconValue::HoconString("hello"));
        assert_eq!(parse(&content), Ok(HoconValue::HoconObject(expected_map)));
    }
}

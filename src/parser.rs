use std::collections::HashMap;

use nom::{
    branch::alt, bytes::complete::{tag, take_while}, character::complete::{alphanumeric0, char}, combinator::{map, peek, value}, multi::{many0, many_m_n}, sequence::{delimited, tuple, Tuple}, IResult
};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum HoconValue {
    HoconString(String),
    HoconNumber(f64),
    HoconObject(HashMap<String, HoconValue>),
    HoconArray(Vec<HoconValue>),
    HoconBoolean(bool),
    HoconNull,
}

#[derive(Error, Debug, PartialEq)]
pub enum HoconError {
    #[error("Parse error")]
    ParseError
}

pub fn parse<'a>(input: &'a str) -> Result<HoconValue, HoconError> {
    let r = parse_object(input);
    match r {
        Ok((_, value)) => {
            Ok(value)
        },
        Err(_) => Err(HoconError::ParseError)
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

    alt((
        delimited(char('"'), parse_str, char('"')),
        parse_str
    ))(input)

}

fn parse_value<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
    alt((
        null,
        boolean,
        map(string, |s| HoconValue::HoconString(s.to_string()))
    ))(input)
}

fn next_element_whitespace<'a>(input: &'a str) -> IResult<&'a str, ()> {
    map(
        tuple((whitespace, many_m_n(0, 1, char(',')))),
        |_| ()
    )(input)
}

fn key_value<'a>(input: &'a str) -> IResult<&'a str, (&'a str, HoconValue)> {

    fn separator<'a>(input: &'a str) -> IResult<&'a str, ()> {
        map(
            alt((
                char(':'),
                char('='),
                peek(char('{'))
            )),
            |_| ()
        )(input)
    }

    let (input, (_, path, _, _, _, value, _)) = (whitespace, string, whitespace, separator, whitespace, parse_value, next_element_whitespace).parse(input)?;
    Ok((input, (path, value)))
}

fn parse_object_inner<'a>(input: &'a str) -> IResult<&'a str, HoconValue> {
    map(many0(key_value), |kvs| {
        let mut map = HashMap::new();
        for (k, v) in kvs {
            map.insert(k.to_owned(), v.to_owned());
        }
        HoconValue::HoconObject(map)
    })(input)
}

fn parse_object<'a>(input: &'a str) ->IResult<&'a str, HoconValue> {
    alt((
        delimited(char('{'), parse_object_inner, char('}')),
        parse_object_inner
    ))(input)
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
        assert_eq!(key_value("test = true"), Ok(("", ("test", HoconValue::HoconBoolean(true)))));
    }

    #[test]
    fn parse_basic_json_object() {
        let content = r#"{ "hello": "world" }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello".to_string(), HoconValue::HoconString("world".to_string()));
        assert_eq!(
            parse(&content),
            Ok(HoconValue::HoconObject(expected_map))
        );
    }

    #[test]
    fn parse_json_object_with_two_keys() {
        let content = r#"{ "hello": "world", "world": "hello" }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello".to_string(), HoconValue::HoconString("world".to_string()));
        expected_map.insert("world".to_string(), HoconValue::HoconString("hello".to_string()));
        assert_eq!(
            parse(&content),
            Ok(HoconValue::HoconObject(expected_map))
        );
    }

    #[test]
    fn parse_json_object_with_two_keys_multiline() {
        let content = r#"{
            "hello": "world",
            "world": "hello"
        }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello".to_string(), HoconValue::HoconString("world".to_string()));
        expected_map.insert("world".to_string(), HoconValue::HoconString("hello".to_string()));
        assert_eq!(
            parse(&content),
            Ok(HoconValue::HoconObject(expected_map))
        );
    }

    #[test]
    fn parse_hocon_object_with_two_keys() {
        let content = r#"
            hello: "world"
            world: "hello"
        }"#;
        let mut expected_map = HashMap::new();
        expected_map.insert("hello".to_string(), HoconValue::HoconString("world".to_string()));
        expected_map.insert("world".to_string(), HoconValue::HoconString("hello".to_string()));
        assert_eq!(
            parse(&content),
            Ok(HoconValue::HoconObject(expected_map))
        );
    }
}

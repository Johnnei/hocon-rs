use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::char,
    combinator::{all_consuming, map, peek, value},
    error::{convert_error, ErrorKind, ParseError},
    multi::{many0, many1, many_m_n},
    number::complete::double,
    sequence::{delimited, tuple, Tuple},
    AsChar, IResult, InputTakeAtPosition,
};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct HoconObject<'a> {
    pub data: HashMap<String, HoconValue<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HoconInclusion<'a> {
    File(&'a str),
    Url(&'a str),
    Classpath(&'a str),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HoconField<'a> {
    Include(HoconInclusion<'a>),
    KeyValue(&'a str, HoconValue<'a>),
}

/// Represents a hocon value within the AST representation.
#[derive(Clone, Debug, PartialEq)]
pub enum HoconValue<'a> {
    HoconString(&'a str),
    HoconNumber(f64),
    HoconObject(Vec<HoconField<'a>>),
    HoconArray(Vec<HoconValue<'a>>),
    HoconBoolean(bool),
    HoconNull,
    HoconInclude(HoconInclusion<'a>),
}

/// Represents the various modes of failure while parsing or evaluating hocon files.
#[derive(Error, Debug, PartialEq)]
pub enum HoconError {
    // TODO Integrate better with nom error to get better parsing error docs
    #[error("Parse error")]
    ParseError { msg: String },
}

/// Parses the given input as a Hocon document into a Hocon AST.
pub fn parse<'a, E: ParseError<&'a str>>(input: &'a str) -> Result<HoconValue<'a>, HoconError> {
    let r = alt((empty_content, parse_object))(input);
    match r {
        Ok((_, value)) => Ok(value),
        Err(nom::Err::Error(e)) => {
            let msg = convert_error(input, e);
            Err(HoconError::ParseError { msg })
        }
        _ => Err(HoconError::ParseError {
            msg: "Unknown error".to_string(),
        }),
    }
}

fn empty_content<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue, E> {
    map(all_consuming(whitespace), |_| HoconValue::HoconObject(vec![]))(input)
}

fn null<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue, E> {
    let (input, _) = tag("null")(input)?;
    Ok((input, HoconValue::HoconNull))
}

fn boolean<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue, E> {
    let parse_true = value(HoconValue::HoconBoolean(true), tag("true"));
    let parse_false = value(HoconValue::HoconBoolean(false), tag("false"));
    alt((parse_true, parse_false))(input)
}

fn whitespace<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
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

fn string<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    fn is_string_char(c: char) -> bool {
        !(c.is_alphanum() || c == '.')
    }

    fn parse_str<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
        input.split_at_position_complete(is_string_char)
    }

    fn parse_str1<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
        input.split_at_position1_complete(is_string_char, ErrorKind::AlphaNumeric)
    }

    alt((delimited(char('"'), parse_str, char('"')), parse_str1))(input)
}

fn number<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue, E> {
    map(double, HoconValue::HoconNumber)(input)
}

fn include<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconInclusion, E> {
    let (remainder, (_, _, (_, v))) = (
        tag("include"),
        whitespace,
        alt((
            tuple((
                tag("url"),
                delimited(char('('), map(string, HoconInclusion::Url), char(')')),
            )),
            tuple((
                tag("file"),
                delimited(char('('), map(string, HoconInclusion::File), char(')')),
            )),
            tuple((
                tag("classpath"),
                delimited(char('('), map(string, HoconInclusion::Classpath), char(')')),
            )),
        )),
    )
        .parse(input)?;
    Ok((remainder, v))
}

fn parse_value<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    alt((
        null,
        map(include, HoconValue::HoconInclude),
        boolean,
        number,
        array,
        parse_object,
        map(string, HoconValue::HoconString),
    ))(input)
}

fn next_element_whitespace<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    map(tuple((whitespace, many_m_n(0, 1, char(',')))), |_| ())(input)
}

fn key_value<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (&'a str, HoconValue), E> {
    fn separator<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
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

fn object_field<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconField, E> {
    alt((
        map(include, HoconField::Include),
        map(key_value, |(k, v)| HoconField::KeyValue(k, v)),
    ))(input)
}

fn array<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue, E> {
    fn array_element<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue, E> {
        let (input, (_, value, _)) = (whitespace, parse_value, next_element_whitespace).parse(input)?;
        Ok((input, value))
    }

    delimited(char('['), map(many0(array_element), HoconValue::HoconArray), char(']'))(input)
}

fn parse_object<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    fn parse_inner0<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
        map(many0(object_field), HoconValue::HoconObject)(input)
    }

    fn parse_inner1<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
        map(many1(object_field), HoconValue::HoconObject)(input)
    }

    alt((delimited(char('{'), parse_inner0, char('}')), parse_inner1))(input)
}

#[cfg(test)]
mod tests {

    use nom::error::VerboseError;

    use super::*;
    use crate::parser::HoconValue;

    #[test]
    fn test_null() {
        assert_eq!(null::<VerboseError<&str>>("null"), Ok(("", HoconValue::HoconNull)));
    }

    #[test]
    fn test_boolean_true() {
        assert_eq!(
            boolean::<VerboseError<&str>>("true"),
            Ok(("", HoconValue::HoconBoolean(true)))
        );
    }

    #[test]
    fn test_boolean_false() {
        assert_eq!(
            boolean::<VerboseError<&str>>("false"),
            Ok(("", HoconValue::HoconBoolean(false)))
        );
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(whitespace::<VerboseError<&str>>("     test"), Ok(("test", ())));
    }

    #[test]
    fn test_string() {
        assert_eq!(string::<VerboseError<&str>>("test"), Ok(("", "test")));
    }

    #[test]
    fn test_quoted_string() {
        assert_eq!(string::<VerboseError<&str>>("\"test\""), Ok(("", "test")));
    }

    #[test]
    fn test_key_value() {
        assert_eq!(
            key_value::<VerboseError<&str>>("test = true"),
            Ok(("", ("test", HoconValue::HoconBoolean(true))))
        );
    }

    #[test]
    fn test_number() {
        assert_eq!(
            number::<VerboseError<&str>>("42"),
            Ok(("", HoconValue::HoconNumber(42f64)))
        );
    }

    #[test]
    fn test_array() {
        let expected_data = vec![
            HoconValue::HoconNumber(1f64),
            HoconValue::HoconNumber(2f64),
            HoconValue::HoconNumber(3f64),
        ];
        assert_eq!(
            array::<VerboseError<&str>>("[1,2,3]"),
            Ok(("", HoconValue::HoconArray(expected_data)))
        );
    }

    #[test]
    fn test_array_trailing_comma() {
        let expected_data = vec![
            HoconValue::HoconNumber(1f64),
            HoconValue::HoconNumber(2f64),
            HoconValue::HoconNumber(3f64),
        ];
        assert_eq!(
            array::<VerboseError<&str>>("[1,2,3,]"),
            Ok(("", HoconValue::HoconArray(expected_data)))
        );
    }

    #[test]
    fn parse_basic_json_object() {
        let content = r#"{ "hello": "world" }"#;
        let expected = vec![HoconField::KeyValue("hello", HoconValue::HoconString("world"))];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_json_object_with_two_keys() {
        let content = r#"{ "hello": "world", "world": "hello" }"#;
        let expected = vec![
            HoconField::KeyValue("hello", HoconValue::HoconString("world")),
            HoconField::KeyValue("world", HoconValue::HoconString("hello")),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_json_object_with_two_keys_multiline() {
        let content = r#"{
            "hello": "world",
            "world": "hello"
        }"#;
        let expected = vec![
            HoconField::KeyValue("hello", HoconValue::HoconString("world")),
            HoconField::KeyValue("world", HoconValue::HoconString("hello")),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_hocon_object_with_two_keys() {
        let content = r#"{
            hello: "world"
            world: "hello"
        }"#;
        let expected = vec![
            HoconField::KeyValue("hello", HoconValue::HoconString("world")),
            HoconField::KeyValue("world", HoconValue::HoconString("hello")),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_inclusion() {
        let content = r#"include file("test.conf")"#;
        let expected = HoconInclusion::File("test.conf");
        assert_eq!(include::<VerboseError<&str>>(content), Ok(("", expected)));
    }

    #[test]
    fn parse_inclusion_merge() {
        let content = r#"include file("test.conf")
            hello = "world"
        "#;
        let expected = vec![
            HoconField::Include(HoconInclusion::File("test.conf")),
            HoconField::KeyValue("hello", HoconValue::HoconString("world")),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn pares_inclusion_value() {
        let content = r#"
            hello = include file("test.conf")
        "#;
        let expected = vec![HoconField::KeyValue(
            "hello",
            HoconValue::HoconInclude(HoconInclusion::File("test.conf")),
        )];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_empty_line() {
        assert_eq!(empty_content::<VerboseError<&str>>(""), Ok(("", HoconValue::HoconObject(vec![]))));
        assert_eq!(parse::<VerboseError<&str>>(""), Ok(HoconValue::HoconObject(vec![])));
    }

    #[test]
    fn parse_empty_line_whitespace() {
        assert_eq!(parse::<VerboseError<&str>>("   "), Ok(HoconValue::HoconObject(vec![])));
    }

    #[test]
    fn parse_empty_multiline() {
        let content = r#"

        "#;
        let expected = vec![];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }
}

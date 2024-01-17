use std::collections::HashMap;

use nom::{IResult, branch::alt, bytes::complete::tag, combinator::value};

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

#[cfg(test)]
mod tests {

    use crate::parser::HoconValue;
    use super::*;

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

}

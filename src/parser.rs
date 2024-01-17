use std::collections::HashMap;

use nom::{IResult, branch::alt, bytes::complete::tag};

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {

    use crate::parser::HoconValue;
    use super::*;

    #[test]
    fn test_null() {
        assert_eq!(null("null"), Ok(("", HoconValue::HoconNull)));
    }

}

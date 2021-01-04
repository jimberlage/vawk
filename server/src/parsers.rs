use crate::byte_trie::ByteTrie;
use nom::branch::alt;
use nom::bytes::complete::{tag, take};
use nom::character::complete::{digit1, space0};
use nom::combinator::{self, value};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::Finish;
use nom::IResult;
use regex::bytes::Regex;
use std::str::FromStr;

#[derive(Debug)]
pub struct InvalidFieldSeparatorError(String);

#[derive(Debug)]
pub struct InvalidIndexFiltersError(String);

#[derive(Debug)]
pub struct InvalidRegexFilterError(String);

/*********************************************************************************************************************
 * Rules for separating data                                                                                         *
 *                                                                                                                   *
 * Users can choose how they want to split up their result set into lines or columns.                                *
 * The UX is patterned after Unix's IFS (Internal Field Separator), since it will be familiar to users of the tool.  *
 * Users can give a single separator, or any number of separators as a single string (they will be split on the      *
 * empty string.)  However, just an empty string is not treated as a separator, to avoid garbled-looking output.     *
 *********************************************************************************************************************/

/// escaped_separator handles getting escaped characters from a user-input separator string.
/// It will treat "\\n", "\\t", "\\r", and "\\s" as the literal characters '\n', '\t', '\r', and ' '.
fn escaped_field_separator(input: &str) -> IResult<&str, u8> {
    alt((
        value(b'\n', tag("\\n")),
        value(b'\t', tag("\\t")),
        value(b'\r', tag("\\r")),
        value(b' ', tag("\\s")),
    ))(input)
}

fn field_separator<'a>(input: &'a str, byte_trie: &mut ByteTrie) -> IResult<&'a str, ()> {
    combinator::map(
        many0(alt((
            combinator::map(escaped_field_separator, |byte| vec![byte]),
            combinator::map(take(1usize), |s: &str| s.bytes().collect::<Vec<u8>>()),
        ))),
        |mut chars: Vec<Vec<u8>>| {
            let mut combined = vec![];
            for char_bytes in chars.iter_mut() {
                combined.append(char_bytes);
            }

            byte_trie.insert(&combined);
        },
    )(input)
}

/// Parses field separators from a string.
pub fn parse_field_separators(
    string_representations: &Vec<String>,
) -> Result<ByteTrie, InvalidFieldSeparatorError> {
    let mut separators = ByteTrie::new();

    for string_representation in string_representations {
        match field_separator(string_representation, &mut separators).finish() {
            Err(error) => return Err(InvalidFieldSeparatorError(error.input.to_owned())),
            Ok((unconsumed_input, _))
                if separators.is_empty() && !unconsumed_input.is_empty() =>
            {
                return Err(InvalidFieldSeparatorError(unconsumed_input.to_owned()))
            }
            _ => (),
        }
    }

    Ok(separators)
}

/*********************************************************************************************************************
 * Rules for including or excluding data                                                                             *
 *                                                                                                                   *
 * There are two ways to spell out that you only want certain strings to be included or excluded in the result set.  *
 * They are:                                                                                                         *
 * - By index; users can say that they want a particular index, or indices within a range, or some combination.      *
 * - By regex; users can say that they only want lines matching a particular regex.                                  *
 *********************************************************************************************************************/

#[derive(Debug, PartialEq)]
pub enum IndexFilter {
    Bounded(usize, usize),
    LowerBounded(usize),
    UpperBounded(usize),
    Exact(usize),
}

impl IndexFilter {
    pub fn is_match(&self, i: usize) -> bool {
        match self {
            IndexFilter::Bounded(lower, upper) => i >= *lower && i < *upper,
            IndexFilter::LowerBounded(lower) => i >= *lower,
            IndexFilter::UpperBounded(upper) => i < *upper,
            IndexFilter::Exact(j) => i == *j,
        }
    }
}

fn index(input: &str) -> IResult<&str, usize> {
    combinator::map(digit1, |s: &str| usize::from_str(s).unwrap())(input)
}

fn bounded(input: &str) -> IResult<&str, IndexFilter> {
    combinator::map(separated_pair(index, tag(".."), index), |(lower, upper)| {
        IndexFilter::Bounded(lower, upper)
    })(input)
}

fn lower_bounded(input: &str) -> IResult<&str, IndexFilter> {
    combinator::map(terminated(index, tag("..")), |lower| {
        IndexFilter::LowerBounded(lower)
    })(input)
}

fn upper_bounded(input: &str) -> IResult<&str, IndexFilter> {
    combinator::map(preceded(tag(".."), index), |upper| {
        IndexFilter::UpperBounded(upper)
    })(input)
}

fn exact(input: &str) -> IResult<&str, IndexFilter> {
    combinator::map(index, |i| IndexFilter::Exact(i))(input)
}

fn index_filter(input: &str) -> IResult<&str, IndexFilter> {
    alt((bounded, lower_bounded, upper_bounded, exact))(input)
}

fn index_filter_separator(input: &str) -> IResult<&str, ()> {
    combinator::map(delimited(space0, tag(","), space0), |_| ())(input)
}

/// Parses index filters that a user inputs.
///
/// This parses 4 types of index filters:
/// 1. Exact: "4" matches the row with the index of "4".
/// 2. Bounded: "6..10" matches rows where the index is >= 6 and < 10.
/// 3. Lower bounded: "5.." matches rows where the index is >= 5.
/// 4. Upper bounded: "..96" matches rows where the index is < 96.
fn index_filters(input: &str) -> IResult<&str, Vec<IndexFilter>> {
    delimited(
        space0,
        many0(alt((
            combinator::map(tuple((index_filter, index_filter_separator)), |(r, _)| r),
            index_filter,
        ))),
        space0,
    )(input)
}

pub fn parse_index_filters(
    string_representation: &str,
) -> Result<Vec<IndexFilter>, InvalidIndexFiltersError> {
    match index_filters(string_representation).finish() {
        Err(error) => Err(InvalidIndexFiltersError(error.input.to_owned())),
        Ok((unconsumed_input, rules)) if rules.is_empty() && !unconsumed_input.is_empty() => {
            Err(InvalidIndexFiltersError(unconsumed_input.to_owned()))
        }
        Ok((_, rules)) => Ok(rules),
    }
}

pub fn parse_regex_filter(string_representation: &str) -> Result<Regex, InvalidRegexFilterError> {
    Regex::new(string_representation).map_err(|error| InvalidRegexFilterError(format!("{}", error)))
}

#[cfg(test)]
mod test {
    use crate::byte_trie::ByteTrie;

    #[test]
    fn parse_field_separators() {
        let mut expected = ByteTrie::new();
        expected.insert(&[b'\r', b'\n']);
        match super::parse_field_separators(&vec!["\\r\\n".into()]) {
            Ok(actual) => assert_eq!(actual, expected),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn parse_index_filters() {
        let expected = vec![
            super::IndexFilter::Exact(1usize),
            super::IndexFilter::LowerBounded(5usize),
        ];
        match super::parse_index_filters("1, 5..") {
            Ok(actual) => assert_eq!(actual, expected),
            Err(_) => assert!(false),
        }
    }
}

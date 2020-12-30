use byte_trie::{ByteTrie, Membership};
use nom::branch::alt;
use nom::bytes::complete::{tag, take};
use nom::character::complete::{digit1, space0};
use nom::combinator::{self, value};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::Finish;
use nom::IResult;
use regex::bytes::Regex;
use std::result;
use std::str::FromStr;

use crate::byte_trie;

#[derive(Debug)]
pub enum TransformationError {
    InvalidFieldSeparator(String),
    InvalidIndexRules(String),
    InvalidRegexRule(String),
}

type Result<T> = result::Result<T, TransformationError>;

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

fn field_separator(input: &str) -> IResult<&str, ByteTrie> {
    combinator::map(
        many0(alt((
            combinator::map(escaped_field_separator, |byte| vec![byte]),
            combinator::map(take(1usize), |s: &str| s.bytes().collect::<Vec<u8>>()),
        ))),
        |chars: Vec<Vec<u8>>| {
            let mut combined = vec![];
            for char_bytes in chars {
                combined.append(&mut char_bytes);
            }

            let mut result = ByteTrie::new();
            result.insert(&combined);
            result
        },
    )(input)
}

/// Parses field separators from a string.
pub fn parse_field_separator(string_representation: &str) -> Result<ByteTrie> {
    match field_separator(string_representation).finish() {
        Err(error) => Err(TransformationError::InvalidFieldSeparator(
            error.input.to_owned(),
        )),
        Ok((unconsumed_input, separators))
            if separators.is_empty() && !unconsumed_input.is_empty() =>
        {
            Err(TransformationError::InvalidFieldSeparator(
                unconsumed_input.to_owned(),
            ))
        }
        Ok((_, separators)) => Ok(separators),
    }
}

/// Splits string data into parts according to the given separators.
fn split(separators: &ByteTrie, data: Vec<u8>) -> Vec<Vec<u8>> {
    let mut result = vec![];
    let mut current_line = vec![];
    let mut current_separator = vec![];

    for byte in data {
        current_separator.push(byte);
        match separators.membership(current_separator.as_slice()) {
            Membership::NotIncluded => {
                current_line.push(byte);
                if !current_separator.is_empty() {
                    current_separator = vec![];
                }
            }
            Membership::Included if !current_line.is_empty() => {
                result.push(current_line);
                current_line = vec![];
            }
            Membership::Included => (),
            Membership::IncludedAndTerminal if !current_line.is_empty() => {
                result.push(current_line);
                current_line = vec![];
                current_separator = vec![];
            }
            Membership::IncludedAndTerminal => {
                current_separator = vec![];
            }
        };
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    result
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
pub enum IndexRule {
    Bounded(usize, usize),
    LowerBounded(usize),
    UpperBounded(usize),
    Exact(usize),
}

impl IndexRule {
    fn is_match(&self, i: usize) -> bool {
        match self {
            IndexRule::Bounded(lower, upper) => i >= *lower && i < *upper,
            IndexRule::LowerBounded(lower) => i >= *lower,
            IndexRule::UpperBounded(upper) => i < *upper,
            IndexRule::Exact(j) => i == *j,
        }
    }
}

fn index(input: &str) -> IResult<&str, usize> {
    combinator::map(digit1, |s: &str| usize::from_str(s).unwrap())(input)
}

fn bounded(input: &str) -> IResult<&str, IndexRule> {
    combinator::map(separated_pair(index, tag(".."), index), |(lower, upper)| {
        IndexRule::Bounded(lower, upper)
    })(input)
}

fn lower_bounded(input: &str) -> IResult<&str, IndexRule> {
    combinator::map(terminated(index, tag("..")), |lower| {
        IndexRule::LowerBounded(lower)
    })(input)
}

fn upper_bounded(input: &str) -> IResult<&str, IndexRule> {
    combinator::map(preceded(tag(".."), index), |upper| {
        IndexRule::UpperBounded(upper)
    })(input)
}

fn exact(input: &str) -> IResult<&str, IndexRule> {
    combinator::map(index, |i| IndexRule::Exact(i))(input)
}

fn index_rule(input: &str) -> IResult<&str, IndexRule> {
    alt((bounded, lower_bounded, upper_bounded, exact))(input)
}

fn index_rule_separator(input: &str) -> IResult<&str, ()> {
    combinator::map(delimited(space0, tag(","), space0), |_| ())(input)
}

/// Parses index rules that a user inputs.
///
/// This parses 4 types of index rules:
/// 1. Exact: "4" matches the row with the index of "4".
/// 2. Bounded: "6..10" matches rows where the index is >= 6 and < 10.
/// 3. Lower bounded: "5.." matches rows where the index is >= 5.
/// 4. Upper bounded: "..96" matches rows where the index is < 96.
fn index_rules(input: &str) -> IResult<&str, Vec<IndexRule>> {
    delimited(
        space0,
        many0(alt((
            combinator::map(tuple((index_rule, index_rule_separator)), |(r, _)| r),
            index_rule,
        ))),
        space0,
    )(input)
}

fn parse_index_rules(string_representation: &str) -> Result<Vec<IndexRule>> {
    match index_rules(string_representation).finish() {
        Err(error) => Err(TransformationError::InvalidIndexRules(
            error.input.to_owned(),
        )),
        Ok((unconsumed_input, rules)) if rules.is_empty() && !unconsumed_input.is_empty() => Err(
            TransformationError::InvalidIndexRules(unconsumed_input.to_owned()),
        ),
        Ok((_, rules)) => Ok(rules),
    }
}

/// Parse the rules for indexes, then keep only entries in the data that match the rules given for indexes.
///
/// This function is a bit atypical in that the rules_str argument is expected to be user input, and has purposefully relaxed parsing logic.
/// It also returns data even in the error case, so that the user still gets some feedback even with invalid input.
/// This is **not** a goal of the rest of the code, in general failing fast is preferred unless there is a strong tie to user input.
fn keep_index_matches(rules: &Vec<IndexRule>, data: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut result = vec![];

    for i in 0..data.len() {
        if rules.iter().any(|rule| rule.is_match(i)) {
            result.push(data[i].clone());
        }
    }

    result
}

fn parse_regex_rule(string_representation: &str) -> Result<Regex> {
    Regex::new(string_representation)
        .map_err(|error| TransformationError::InvalidRegexRule(format!("{}", error)))
}

fn keep_regex_matches(regex: &Regex, data: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    data.into_iter()
        .filter(|&field| regex.is_match(field.as_slice()))
        .map(|field| field.clone())
        .collect()
}

fn transform_1d(
    separators: &Option<ByteTrie>,
    regex: &Option<Regex>,
    index_rules: &Option<Vec<IndexRule>>,
    data: Vec<u8>,
) -> Vec<Vec<u8>> {
    let mut result = match separators {
        None => vec![data],
        Some(separators) => split(separators, data),
    };

    if let Some(index_rules) = index_rules {
        result = keep_index_matches(index_rules, result);
    }

    if let Some(regex) = regex {
        result = keep_regex_matches(regex, result);
    }

    result
}

pub fn transform_2d(
    line_separators: &Option<ByteTrie>,
    line_regex: &Option<Regex>,
    line_index_rules: &Option<Vec<IndexRule>>,
    row_separators: &Option<ByteTrie>,
    row_regex: &Option<Regex>,
    row_index_rules: &Option<Vec<IndexRule>>,
    data: Vec<u8>,
) -> Vec<Vec<Vec<u8>>> {
    transform_1d(line_separators, line_regex, line_index_rules, data)
        .into_iter()
        .map(|line| transform_1d(row_separators, row_regex, row_index_rules, line))
        .collect()
}

#[cfg(test)]
mod test {
    use crate::byte_trie::ByteTrie;
    use regex::bytes::Regex;

    fn bytes_vec(data: Vec<&str>) -> Vec<Vec<u8>> {
        data.into_iter().map(|s| s.bytes().collect()).collect()
    }

    #[test]
    fn parse_field_separator() {
        let mut expected = ByteTrie::new();
        expected.insert(&[b'\r', b'\n']);
        match super::parse_field_separator("\\r\\n") {
            Ok(actual) => assert_eq!(actual, expected),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn split() {
        // Special characters are parsed correctly.
        let mut separators = ByteTrie::new();
        separators.insert(&[b'\n']);
        let expected: Vec<Vec<u8>> = bytes_vec(vec!["hi\tthere\tthis", "could\tbe\tcsv"]);
        let actual = super::split(
            &separators,
            "hi\tthere\tthis\ncould\tbe\tcsv".bytes().collect(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_index_rules() {
        let expected = vec![
            super::IndexRule::Exact(1usize),
            super::IndexRule::LowerBounded(5usize),
        ];
        match super::parse_index_rules("1, 5..") {
            Ok(actual) => assert_eq!(actual, expected),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn keep_index_matches() {
        // The rule "1, 5.." keeps indexes 1, 5, 6, 7, 8.
        let data: Vec<Vec<u8>> = bytes_vec(vec![
            "The", "quick", "brown", "fox", "jumped", "over", "the", "lazy", "dog",
        ]);
        let expected: Vec<Vec<u8>> = bytes_vec(vec!["quick", "over", "the", "lazy", "dog"]);
        let actual = super::keep_index_matches(
            &vec![
                super::IndexRule::Exact(1usize),
                super::IndexRule::LowerBounded(5usize),
            ],
            data,
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn keep_regex_matches() {
        // Special characters are parsed correctly.
        let regex = Regex::new("3[0-9]{3}").unwrap();
        let expected = bytes_vec(vec![
            "SystemUIS\t343\tjimberlage\t5u\tIPv4\t0t0\tUDP\t*:3100",
            "rapportd\t379\tjimberlage\t4u\tIPv4\t0t0\tTCP\t*:3001 (LISTEN)",
            "rapportd\t379\tjimberlage\t5u\tIPv6\t0t0\tTCP\t*:3005 (LISTEN)",
        ]);
        let actual = super::keep_regex_matches(
            &regex,
            bytes_vec(vec![
                "COMMAND\tPID\tUSER\tFD\tTYPE\tSIZE/OFF\tNODE\tNAME",
                "loginwind\t168\tjimberlage\t7u\tIPv4\t0t0\tUDP\t*:5678",
                "SystemUIS\t343\tjimberlage\t5u\tIPv4\t0t0\tUDP\t*:3100",
                "SystemUIS\t343\tjimberlage\t8u\tIPv4\t0t0\tUDP\t*:9004",
                "rapportd\t379\tjimberlage\t4u\tIPv4\t0t0\tTCP\t*:3001 (LISTEN)",
                "rapportd\t379\tjimberlage\t5u\tIPv6\t0t0\tTCP\t*:3005 (LISTEN)",
            ]),
        );
        assert_eq!(actual, expected);
    }
}

use nom::branch::alt;
use nom::bytes::complete::{tag, take};
use nom::character::complete::{digit1, space0};
use nom::combinator::{self, value};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::Finish;
use nom::IResult;
use regex::{self, Regex};
use std::collections::HashSet;
use std::result;
use std::str::FromStr;

#[derive(Debug)]
pub enum TransformationError {
    InvalidFieldSeparator(String),
    InvalidIndexRules(String),
    InvalidRegexRule(String),
}

type Result<T> = result::Result<T, (Vec<String>, TransformationError)>;

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
fn escaped_field_separator(input: &str) -> IResult<&str, char> {
    alt((
        value('\n', tag("\\n")),
        value('\t', tag("\\t")),
        value('\r', tag("\\r")),
        value(' ', tag("\\s")),
    ))(input)
}

fn field_separators(input: &str) -> IResult<&str, HashSet<char>> {
    combinator::map(
        many0(alt((
            escaped_field_separator,
            combinator::map(take(1usize), |s: &str| s.chars().next().unwrap()),
        ))),
        |separators| separators.into_iter().collect(),
    )(input)
}

/// splits string data into parts according to the given separators.
fn split(separators_str: &str, data: &str) -> Result<Vec<String>> {
    match field_separators(separators_str).finish() {
        Err(error) => Err((
            vec![data.to_owned()],
            TransformationError::InvalidFieldSeparator(error.input.to_owned()),
        )),
        Ok((unconsumed_input, separators))
            if separators.is_empty() && unconsumed_input.is_empty() =>
        {
            Ok(vec![data.to_owned()])
        }
        Ok((unconsumed_input, separators)) if separators.is_empty() => Err((
            vec![data.to_owned()],
            TransformationError::InvalidFieldSeparator(unconsumed_input.to_owned()),
        )),
        Ok((unconsumed_input, separators)) => {
            let mut result = vec![];
            let mut current_line = vec![];

            for c in data.chars() {
                if separators.contains(&c) {
                    if current_line.len() > 0 {
                        result.push(current_line.into_iter().collect());
                        current_line = vec![];
                    }
                } else {
                    current_line.push(c);
                }
            }

            if current_line.len() > 0 {
                result.push(current_line.into_iter().collect());
            }

            if unconsumed_input.is_empty() {
                Ok(result)
            } else {
                Err((
                    result,
                    TransformationError::InvalidIndexRules(unconsumed_input.to_owned()),
                ))
            }
        }
    }
}

/*********************************************************************************************************************
 * Rules for including or excluding data                                                                             *
 *                                                                                                                   *
 * There are two ways to spell out that you only want certain strings to be included or excluded in the result set.  *
 * They are:                                                                                                         *
 * - By index; users can say that they want a particular index, or indices within a range, or some combination.      *
 * - By regex; users can say that they only want lines matching a particular regex.                                  *
 *********************************************************************************************************************/

enum IndexRule {
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

/// Parse the rules for indexes, then keep only entries in the data that match the rules given for indexes.
///
/// This function is a bit atypical in that the rules_str argument is expected to be user input, and has purposefully relaxed parsing logic.
/// It also returns data even in the error case, so that the user still gets some feedback even with invalid input.
/// This is **not** a goal of the rest of the code, in general failing fast is preferred unless there is a strong tie to user input.
fn keep_index_matches(rules_str: &str, data: Vec<String>) -> Result<Vec<String>> {
    match index_rules(rules_str).finish() {
        Err(error) => Err((
            data,
            TransformationError::InvalidIndexRules(error.input.to_owned()),
        )),
        Ok((unconsumed_input, rules)) if rules.is_empty() && unconsumed_input.is_empty() => {
            Ok(data)
        }
        Ok((unconsumed_input, rules)) if rules.is_empty() => Err((
            data,
            TransformationError::InvalidIndexRules(unconsumed_input.to_owned()),
        )),
        Ok((unconsumed_input, rules)) => {
            let mut result = vec![];

            for i in 0..data.len() {
                if rules.iter().any(|rule| rule.is_match(i)) {
                    result.push(data[i].clone());
                }
            }

            if unconsumed_input.is_empty() {
                Ok(result)
            } else {
                Err((
                    result,
                    TransformationError::InvalidIndexRules(unconsumed_input.to_owned()),
                ))
            }
        }
    }
}

fn keep_regex_matches(regex_str: &str, data: Vec<String>) -> Result<Vec<String>> {
    match Regex::new(regex_str) {
        Err(error) => Err((
            data,
            TransformationError::InvalidRegexRule(format!("{}", error)),
        )),
        Ok(regex) => Ok(data
            .iter()
            .filter(|&field| regex.is_match(field))
            .map(|field| field.clone())
            .collect()),
    }
}

pub fn transform(separators_str: &str, regex_str: &str, rules_str: &str, data: &str) -> Result<Vec<String>> {
    let fields = split(separators_str, data)?;
    let result = keep_index_matches(rules_str, fields)?;
    keep_regex_matches(regex_str, result)
}

#[cfg(test)]
mod test {
    fn owned_string_vec(data: Vec<&str>) -> Vec<String> {
        data.into_iter().map(|s| s.to_owned()).collect()
    }

    #[test]
    fn split() {
        // Special characters are parsed correctly.
        match super::split("\\n\\t", "hi\tthere\tthis\ncould\tbe\tcsv") {
            Ok(actual) => assert_eq!(
                actual,
                owned_string_vec(vec!["hi", "there", "this", "could", "be", "csv"])
            ),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn keep_index_matches() {
        // The rule "1, 5.." keeps indexes 1, 5, 6, 7, 8.
        match super::keep_index_matches(
            "1, 5..",
            owned_string_vec(vec![
                "The", "quick", "brown", "fox", "jumped", "over", "the", "lazy", "dog",
            ]),
        ) {
            Ok(actual) => assert_eq!(
                actual,
                owned_string_vec(vec!["quick", "over", "the", "lazy", "dog"])
            ),
            Err(_) => assert!(false),
        }

        // If the rule is not valid, it is returned in the error.
        match super::keep_index_matches(
            "thisisnotavalidrule",
            owned_string_vec(vec!["one", "two", "three"]),
        ) {
            Err((_, super::TransformationError::InvalidIndexRules(bad_input))) => {
                assert_eq!("thisisnotavalidrule".to_owned(), bad_input)
            }
            _ => assert!(false),
        }

        // The function returns partial results in the case of poor user input, and gives enough data to show the user where parsing failed.
        match super::keep_index_matches(
            "   1..3, 0fdgdg ",
            owned_string_vec(vec!["one", "two", "three", "four", "five", "six"]),
        ) {
            Err((actual, super::TransformationError::InvalidIndexRules(bad_input))) => {
                assert_eq!(actual, owned_string_vec(vec!["one", "two", "three"]));
                assert_eq!("fdgdg ".to_owned(), bad_input)
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn keep_regex_matches() {
        // Special characters are parsed correctly.
        match super::keep_regex_matches(
            "3[0-9]{3}", 
            owned_string_vec(vec![
                "COMMAND\tPID\tUSER\tFD\tTYPE\tSIZE/OFF\tNODE\tNAME",
                "loginwind\t168\tjimberlage\t7u\tIPv4\t0t0\tUDP\t*:5678",
                "SystemUIS\t343\tjimberlage\t5u\tIPv4\t0t0\tUDP\t*:3100",
                "SystemUIS\t343\tjimberlage\t8u\tIPv4\t0t0\tUDP\t*:9004",
                "rapportd\t379\tjimberlage\t4u\tIPv4\t0t0\tTCP\t*:3001 (LISTEN)",
                "rapportd\t379\tjimberlage\t5u\tIPv6\t0t0\tTCP\t*:3005 (LISTEN)"
            ])) {
            Ok(actual) => assert_eq!(
                actual,
                owned_string_vec(vec![
                    "SystemUIS\t343\tjimberlage\t5u\tIPv4\t0t0\tUDP\t*:3100",
                    "rapportd\t379\tjimberlage\t4u\tIPv4\t0t0\tTCP\t*:3001 (LISTEN)",
                    "rapportd\t379\tjimberlage\t5u\tIPv6\t0t0\tTCP\t*:3005 (LISTEN)"
                ])
            ),
            Err(_) => assert!(false),
        }   
    }
}

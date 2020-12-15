use nom::branch::alt;
use nom::bytes::complete::{tag, take};
use nom::character::complete::{digit1, space0};
use nom::combinator::{self, value};
use nom::multi::many0;
use nom::sequence::{preceded, separated_pair, terminated, tuple};
use nom::IResult;
use std::collections::HashSet;
use std::str::FromStr;

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

fn split(separators_str: &str, data: &str) -> Vec<String> {
    let (_, separators) = field_separators(separators_str).unwrap();
    if separators_str.is_empty() {
        return vec![data.to_owned()];
    }

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

    if current_line.len() == 0 {
        result.push(current_line.into_iter().collect());
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
    combinator::map(
        separated_pair(index, tag(".."), index),
        |(lower, upper)| IndexRule::Bounded(lower, upper),
    )(input)
}

fn lower_bounded(input: &str) -> IResult<&str, IndexRule> {
    combinator::map(
        terminated(index, tag("..")),
        |lower| IndexRule::LowerBounded(lower),
    )(input)
}

fn upper_bounded(input: &str) -> IResult<&str, IndexRule> {
    combinator::map(
        preceded(tag(".."), index),
        |upper| IndexRule::UpperBounded(upper),
    )(input)
}

fn exact(input: &str) -> IResult<&str, IndexRule> {
    combinator::map(index,|i| IndexRule::Exact(i))(input)
}

fn index_rule(input: &str) -> IResult<&str, IndexRule> {
    alt((bounded, lower_bounded, upper_bounded, exact))(input)
}

fn index_rule_separator(input: &str) -> IResult<&str, ()> {
    combinator::map(tuple((space0, tag(","), space0)), |_| ())(input)
}

fn index_rules(input: &str) -> IResult<&str, Vec<IndexRule>> {
    many0(alt((
        combinator::map(tuple((index_rule, index_rule_separator)), |(r, _)| r),
        index_rule,
    )))(input)
}

fn keep_index_matches(rules_str: &str, data: &Vec<String>) -> Vec<String> {
    let (_, rules) = index_rules(rules_str).unwrap();
    if rules.is_empty() {
        return data.iter().map(|d| d.clone()).collect();
    }

    let mut result = vec![];

    for i in 0..data.len() {
        if rules.iter().any(|rule| rule.is_match(i)) {
            result.push(data[i].clone());
        }
    }

    result
}

pub fn transform(separators_str: &str, rules_str: &str, data: &str) -> Vec<String> {
    keep_index_matches(rules_str, &split(separators_str, data))
}

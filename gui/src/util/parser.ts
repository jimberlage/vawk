/**
 * Contains rules for parsing string data into a table format.
 */

/*********************************************************************************************************************
 * Rules for separating data                                                                                         *
 *                                                                                                                   *
 * Users can choose how they want to split up their result set into lines or columns.                                *
 * The UX is patterned after Unix's IFS (Internal Field Separator), since it will be familiar to users of the tool.  *
 * Users can give a single separator, or any number of separators as a single string (they will be split on the      *
 * empty string.)  However, just an empty string is not treated as a separator, to avoid garbled-looking output.     *
 *********************************************************************************************************************/

/**
 * 
 * @param separatorsStr 
 */
let parseSeparators = (separatorsStr: string): string[] => {
    return separatorsStr.replace("\\n", "\n").replace("\\t", "\t").replace("\\s", " ").split("");
};

/**
 * 
 * @param separators 
 * @param data 
 */
let splitLines = (separators: string[], data: string): string[] => {
    let result = [];
    let currentLine = [];

    for (let ch of data) {
        if (separators.includes(ch)) {
            if (currentLine.length === 0) {
                result.push(currentLine.join(""));
            }
        } else {
            currentLine.push(ch);
        }
    }

    if (currentLine.length === 0) {
        result.push(currentLine.join(""));
    }

    return result;
}

/*********************************************************************************************************************
 * Rules for including or excluding data                                                                             *
 *                                                                                                                   *
 * There are two ways to spell out that you only want certain strings to be included or excluded in the result set.  *
 * They are:                                                                                                         *
 * - By index; users can say that they want a particular index, or indices within a range, or some combination.      *
 * - By regex; users can say that they only want lines matching a particular regex.                                  *
 *********************************************************************************************************************/

type RangeRule = (index: number) => boolean;

const ruleSeparatorMatcher = /(\s+)?,(\s+)?/;
const rangeMatcher = /(?<lowerBound>\d+)?..(?<upperBound>\d+)?/;
const indexMatcher = /\d+/;

/**
 * Parses a set of rules for splitting a string.
 * Accepts a single index, or a bounded or unbounded range.
 * Rules may be combined with commas.
 * For example, "1, 3..5, 9.." on a line with 11 parts would match lines [1, 3, 4, 9, 10].
 * @param ruleStr A string of individual numbers and ranges that show where to split another string.
 */
let parseIndexRules = (ruleStr: string): RangeRule[] => {
    let individualRules = ruleStr.split(ruleSeparatorMatcher);
    let result = [];

    for (let rule of individualRules) {
        let rangeMatches = rangeMatcher.exec(rule);
        if (rangeMatches && rangeMatches.length === 3) {
            // Rules like "0..3" mean all rows >= 0 and < 3.
            let lowerBound = parseInt(rangeMatches?.groups?.lowerBound as string);
            let upperBound = parseInt(rangeMatches?.groups?.upperBound as string);
            result.push((index: number) => index >= lowerBound && index < upperBound);
        } else if (rangeMatches && rangeMatches?.groups?.lowerBound) {
            // Rules like "5.." mean all rows >= 5.
            let lowerBound = parseInt(rangeMatches?.groups.lowerBound);
            result.push((index: number) => index >= lowerBound);
        } else if (rangeMatches && rangeMatches?.groups?.upperBound) {
            // Rules like "..19" mean all rows < 19.
            let upperBound = parseInt(rangeMatches?.groups.upperBound);
            result.push((index: number) => index >= upperBound);
        } else if (indexMatcher.exec(rule)) {
            // Rules like "40" mean the row with an index of 40.
            let specificIndex = parseInt(rule);
            result.push((index: number) => index === specificIndex);
        }
    }

    return result;
};

/**
 * 
 * @param regexStr 
 */
let parseRegex = (regexStr: string): RegExp => {
    return new RegExp(regexStr)
};

export {};

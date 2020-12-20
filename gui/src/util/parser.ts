/**
 * This module contains logic for parsing string data into a table format.
 *
 * It primarily exposes the transformData function, which splits a row up according to some basic rules.
 * Documentation on the rules and the overall design can be found in the comment blocks below; usage examples can be
 * found in the function docstrings.
 */

let transformData = (separatorStr: string, regexStr: string, indexRulesStr: string, data: string): string[] => {
    // TODO: Switch over to server-side parsing
    return [data];
}

export {
    transformData,
};

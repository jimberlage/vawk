export class UnexpectedAPIResponseError extends Error {
    constructor() {
      super('The server returned an error in response to this request.');
    }
}

export let changeLineIndices = async (
    clientId: string,
    oldIndices: string | undefined,
    indices: string | undefined,
    setIndices: React.Dispatch<React.SetStateAction<string | undefined>>,
    setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
    setIndices(indices);
    let response = await fetch('http://localhost:6846/api/line-index-filters', {
        method: 'put',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            'client_id': clientId,
            'filters': indices,
        }),
    });
    if (response.status !== 200) {
        console.error(response);
        setIndices(oldIndices);
        setError(new UnexpectedAPIResponseError());
    }
};

export let changeLineRegex = async (
    clientId: string,
    oldRegex: string | undefined,
    regex: string | undefined,
    setRegex: React.Dispatch<React.SetStateAction<string | undefined>>,
    setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
    setRegex(regex);
    let response = await fetch('http://localhost:6846/api/line-regex-filter', {
        method: 'put',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            'client_id': clientId,
            'filter': regex,
        }),
    });
    if (response.status !== 200) {
        console.error(response);
        setRegex(oldRegex);
        setError(new UnexpectedAPIResponseError());
    }
};

export let changeLineSeparators = async (
    clientId: string,
    oldSeparators: string[] | undefined,
    separators: string[] | undefined,
    setSeparators: React.Dispatch<React.SetStateAction<string[] | undefined>>,
    setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
    setSeparators(separators);
    let response = await fetch('http://localhost:6846/api/line-separators', {
        method: 'put',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            'client_id': clientId,
            'separators': separators,
        }),
    });
    if (response.status !== 200) {
        console.error(response);
        setSeparators(oldSeparators);
        setError(new UnexpectedAPIResponseError());
    }
};

export let changeRowIndices = async (
    clientId: string,
    oldIndices: string | undefined,
    indices: string | undefined,
    setIndices: React.Dispatch<React.SetStateAction<string | undefined>>,
    setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
    setIndices(indices);
    let response = await fetch('http://localhost:6846/api/row-index-filters', {
        method: 'put',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            'client_id': clientId,
            'filters': indices,
        }),
    });
    if (response.status !== 200) {
        console.error(response);
        setIndices(oldIndices);
        setError(new UnexpectedAPIResponseError());
    }
};

export let changeRowRegex = async (
    clientId: string,
    oldRegex: string | undefined,
    regex: string | undefined,
    setRegex: React.Dispatch<React.SetStateAction<string | undefined>>,
    setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
    setRegex(regex);
    let response = await fetch('http://localhost:6846/api/row-regex-filter', {
        method: 'put',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            'client_id': clientId,
            'filter': regex,
        }),
    });
    if (response.status !== 200) {
        console.error(response);
        setRegex(oldRegex);
        setError(new UnexpectedAPIResponseError());
    }
};

export let changeRowSeparators = async (
    clientId: string,
    oldSeparators: string[] | undefined,
    separators: string[] | undefined,
    setSeparators: React.Dispatch<React.SetStateAction<string[] | undefined>>,
    setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
    setSeparators(separators);
    let response = await fetch('http://localhost:6846/api/row-separators', {
        method: 'put',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            'client_id': clientId,
            'separators': separators,
        }),
    });
    if (response.status !== 200) {
        console.error(response);
        setSeparators(oldSeparators);
        setError(new UnexpectedAPIResponseError());
    }
};

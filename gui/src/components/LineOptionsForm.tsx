import React from 'react';
import { Input } from 'antd';

type LineOptionsFormProps = {
    setIndices: (indicesStr: string) => void;
    separators: string,
    setSeparators: (separatorsStr: string) => void;
    setRegex: (regexStr: string) => void;
};

let LineOptionsForm = (props: LineOptionsFormProps) => {
    return (
        <>
            <Input defaultValue={props.separators}
                   onChange={(event) => props.setSeparators(event.target.value)} />
            <Input onChange={(event) => props.setRegex(event.target.value)} />
            <Input onChange={(event) => props.setIndices(event.target.value)} />
        </>
    );
};

export default LineOptionsForm;
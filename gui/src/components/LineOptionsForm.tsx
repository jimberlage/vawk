import React from 'react';
import { Input } from 'antd';

type LineOptionsFormProps = {
    setInternalFieldSeparator: (ifs: string) => void;
    setLines: (lines: string[]) => void;
};

let LineOptionsForm = (props: LineOptionsFormProps) => {
    return (
        <>
            <Input onChange={(event) => props.setInternalFieldSeparator(event.target.value)} />
        </>
    );
};

export default LineOptionsForm;
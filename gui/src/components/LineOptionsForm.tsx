import { Input } from 'antd';
import { changeLineIndices, changeLineRegex, changeLineSeparators } from './requests';

type LineOptionsFormProps = {
    clientId: string,
    indices: string | undefined,
    regex: string | undefined,
    separators: string[] | undefined,
    setIndices: React.Dispatch<React.SetStateAction<string | undefined>>;
    setRegex: React.Dispatch<React.SetStateAction<string | undefined>>;
    setSeparators: React.Dispatch<React.SetStateAction<string[] | undefined>>;
    setError: React.Dispatch<React.SetStateAction<Error | undefined>>;
};

let LineOptionsForm = (props: LineOptionsFormProps) => {
    return (
        <>
            <Input defaultValue={props.separators && props.separators.length > 0 ? props.separators[0] : ""}
                   onChange={(event) => changeLineSeparators(props.clientId, props.separators, [event.target.value], props.setSeparators, props.setError)} />
            <Input onChange={(event) => changeLineRegex(props.clientId, props.regex, event.target.value, props.setRegex, props.setError)} />
            <Input onChange={(event) => changeLineIndices(props.clientId, props.indices, event.target.value, props.setIndices, props.setError)} />
        </>
    );
};

export default LineOptionsForm;
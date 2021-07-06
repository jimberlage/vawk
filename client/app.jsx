import Papa from 'papaparse';
import * as React from 'react';
import * as ReactDOM from 'react-dom';
import { DetailsList, PrimaryButton, Stack, TextField, ThemeProvider } from '@fluentui/react';
import { FromClient, FromServer, RunCommand } from './definitions_pb.js';

const serializeRunCommandMessage = (command) => {
    const result = new FromClient();
    const message = new RunCommand();
    message.setCommand(command);
    result.setRunCommand(message);
    return result.serializeBinary();
}

const Command = ({ connection }) => {
    const [command, setCommand] = React.useState('');

    return (
        <Stack horizontal={false} style={{flex: 1}}>
            <TextField
                defaultValue={command}
                multiline={true}
                onChange={(event) => {
                    setCommand(event.target.value);
                }}
                rows={10}
            />
            <PrimaryButton
                onClick={(_event) => {
                    connection.send(serializeRunCommandMessage(command).buffer);
                }}
                text='Run'
            />
        </Stack>
    );
}

const Sidebar = ({ connection }) => (
    <div style={{display: 'flex', flex: 1, minWidth: '256px'}}>
        <Command connection={connection} />
    </div>
);

const Table = ({ rows }) => (
    <table>
        <tbody>
            {rows.map((row, i) => (
                <tr key={`${row.join()}:${i}`}>
                    {row.map((cell, j) => (
                        <td key={`${cell}:${j}`} style={{whiteSpace: 'pre'}}>
                            {cell}
                        </td>
                    ))}
                </tr>
            ))}
        </tbody>
    </table>
);

const bytesDecoder = new TextDecoder();

const handleMessage = async (messageEvent, setRows) => {
    const buffer = await messageEvent.data.arrayBuffer();
    const message = FromServer.deserializeBinary(buffer);
    if (message.hasUnexpectedError()) {
        throw new Error(message.getUnexpectedError().getDescription());
    }

    const stdout = bytesDecoder.decode(message.getCompletedCommand().getStdout()).replace(/\n*$/, "");
    const parsedStdout = Papa.parse(stdout, {header: false, delimiter: ',', newline: '\n'});

    setRows(parsedStdout.data);
}

const App = () => {
    const [connection, setConnection] = React.useState();
    const [rows, setRows] = React.useState([]);

    // Set up the connection.
    React.useEffect(() => {
        if (!connection) {
            const newConnection = new WebSocket(`ws://${window.location.host}/ws/`);
            newConnection.onopen = (_event) => {
                setConnection(newConnection);
            };
            newConnection.onclose = (_event) => {
                setConnection(undefined);
            };
            newConnection.onerror = (event) => {
                console.error(event);
                setConnection(undefined);
            };
            newConnection.onmessage = (messageEvent) => {
                handleMessage(messageEvent, setRows).catch((error) => console.error(error));
            };
        }
    }, [connection]);

    // TODO: Only show the table when the connection is ready.
    return (
        <ThemeProvider>
            <Stack horizontal={true}>
                <Stack.Item grow={4}>
                    <Table rows={rows} />
                </Stack.Item>
                <Stack.Item grow={1}>
                    <Sidebar connection={connection} />
                </Stack.Item>
            </Stack>
        </ThemeProvider>
    );
};

ReactDOM.render(<App />, document.getElementById('app'));

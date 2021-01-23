import React, { useEffect, useState } from 'react';
import { Button, Form, Input, Tabs } from 'antd';
import LineOptionsForm from './LineOptionsForm';
import { OutputMessage, addChunk, isComplete, combineStdoutChunks, combineStderrChunks } from '../parser';
import { changeLineSeparators, changeRowSeparators } from './requests';
import 'antd/dist/antd.css';

class InvalidServerEventError extends Error {
  constructor() {
    super('The server sent an invalid event over the wire.');
  }
}

type RowProps = {
  index: number;
  row: string[];
};

let initializeLineSeparators = async (
  clientId: string, 
  setLineSeparators: React.Dispatch<React.SetStateAction<string[] | undefined>>,
  setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
  await changeLineSeparators(clientId, undefined, ['\\n'], setLineSeparators, setError);
};

let initializeRowSeparators = async (
  clientId: string, 
  setRowSeparators: React.Dispatch<React.SetStateAction<string[] | undefined>>,
  setError: React.Dispatch<React.SetStateAction<Error | undefined>>,
) => {
  await changeRowSeparators(clientId, undefined, ['\\n'], setRowSeparators, setError);
};

let Row = (props: RowProps) => {
  return (
    <tr className="whitespace-pre-wrap">
      {props.row.map((cell, index) => <td key={`${index}:${cell}`}>{cell}</td>)}
    </tr>
  );
};

type changeCommandFormValues = {
  'client_id': string | undefined;
  command: string;
}

let changeCommand = (clientId: string, values: changeCommandFormValues) => {
  values['client_id'] = clientId;
  // TODO: Error handling
  fetch('http://localhost:6846/api/command/run', {
    method: 'post',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(values)
  })
};

type ChangeCommandFormProps = {
  clientId: string;
}

let ChangeCommandForm = ({ clientId }: ChangeCommandFormProps) => {
  return (
    <>
      <Form layout="inline" onFinish={(values) => changeCommand(clientId, values)}>
        <Form.Item label="Command" name="command">
          <Input />
        </Form.Item>
        <Form.Item>
          <Button type="primary" htmlType="submit">Submit</Button>
        </Form.Item>
      </Form>
    </>
  )
}

let App = () => {
  // Initialize the app, resetting the time the server has last checked to ensure that it submits an update.
  const [clientId, setClientId] = useState<string | undefined>();
  const [initStatus, setInitStatus] = useState<'uninitialized' | 'pending' | 'initialized'>('uninitialized');

  useEffect(() => {
    (async () => {
      if (initStatus !== 'uninitialized')
        return;
  
      setInitStatus('pending');
  
      try {
        let response = await fetch('http://localhost:6846/api/connect', {
          method: 'get',
          headers: {
            'Content-Type': 'application/json'
          }
        });
        if (response.status !== 200) {
          // TODO: Set error
          return;
        }

        let body = await response.json();
        let clientId = body['client_id'] as string;
        if (clientId) {
          setClientId(clientId);
        }
      } finally {
        setInitStatus('initialized');
      }
    })()
  }, [initStatus, setInitStatus, setClientId]);

  // Our default IFS is a newline character, but that can be changed at the user level.
  const [lineSeparators, setLineSeparators] = useState<string[] | undefined>(['\\n']);
  const [lineRegex, setLineRegex] = useState<string | undefined>('');
  const [lineIndices, setLineIndices] = useState<string | undefined>('');
  const [rowSeparators, setRowSeparators] = useState<string[] | undefined>(['\\s']);
  const [rowRegex, setRowRegex] = useState<string | undefined>('');
  const [rowIndices, setRowIndices] = useState<string | undefined>('');

  const [updateStream, setUpdateStream] = useState<EventSource | undefined>(undefined);
  // Manages our current line buffer.
  const [stdout, setStdout] = useState<string[][] | undefined>(undefined);
  const [stderr, setStderr] = useState<string | undefined>(undefined);
  const [stdoutMessage, setStdoutMessage] = useState<OutputMessage | undefined>(undefined);
  const [stderrMessage, setStderrMessage] = useState<OutputMessage | undefined>(undefined);
  // Allow for errors to be bubbled up.
  const [error, setError] = useState<Error | undefined>();

  // Ensure that the update stream is closed when cleaned up.
  useEffect(() => {
    if (updateStream) {
      return () => updateStream.close();
    }
  }, [updateStream]);

  // Listen for updates when the app is loaded (and cleanup after ourselves).
  useEffect(() => {
    (async () => {
      if (clientId && !updateStream) {
        await Promise.all([
          initializeLineSeparators(clientId, setLineSeparators, setError),
          initializeRowSeparators(clientId, setRowSeparators, setError),
        ]);

        let source = new EventSource(`http://localhost:6846/api/listen?client_id=${clientId}`);
        source.addEventListener('stdout', (event) => {
          if (!(event as MessageEvent)?.data) {
            setError(new InvalidServerEventError());
            return
          }

          // TODO: Use correct error type (InvalidServerEventError)
          let newStdoutMessage = addChunk((event as MessageEvent).data, stdoutMessage);
          if (isComplete(newStdoutMessage)) {
            setStdout(combineStdoutChunks(newStdoutMessage));
            setStdoutMessage(undefined);
          } else {
            setStdoutMessage(newStdoutMessage);
          }
        });

        source.addEventListener('stderr', (event) => {
          if (!(event as MessageEvent)?.data) {
            setError(new InvalidServerEventError());
            return
          }

          let newStderrMessage = addChunk((event as MessageEvent).data, stderrMessage);
          if (isComplete(newStderrMessage)) {
            setStderr(combineStderrChunks(newStderrMessage));
            setStderrMessage(undefined);
          } else {
            setStderrMessage(newStderrMessage);
          }
        });

        setUpdateStream(source);
      }
    })()
  }, [clientId, updateStream, stdoutMessage, stderrMessage, setUpdateStream, setStdoutMessage, setStderrMessage, setStdout, setStderr, setLineSeparators, setRowSeparators, setError]);

  return (
    <>
      <section className="flex flex-row h-screen">
        <main className="w-3/4 overflow-y-scroll">
          <div className="overflow-x-scroll">
            <Tabs defaultActiveKey="stdout">
              <Tabs.TabPane tab="stdout" key="stdout">
                {stdout ?
                  <table className="font-mono table-auto">
                    <thead></thead>
                    <tbody>
                      {stdout.map((row, index) => (
                        <Row key={`${index}:${row}`} row={row} index={index} />
                      ))}
                    </tbody>
                  </table>
                  :
                  <p>
                    No data to show
                  </p>
                }
              </Tabs.TabPane>
              <Tabs.TabPane tab="stderr" key="stderr">
              </Tabs.TabPane>
            </Tabs>
          </div>
        </main>
        <aside className="w-1/4">
          {clientId ? <ChangeCommandForm clientId={clientId} /> : null }
          {clientId ?
            <LineOptionsForm clientId={clientId}
                             indices={lineIndices}
                             regex={lineRegex}
                             separators={lineSeparators}
                             setIndices={setLineIndices}
                             setRegex={setLineRegex}
                             setSeparators={setLineSeparators}
                             setError={setError} />
            : null}
        </aside>
      </section>
    </>
  );
}

export default App;

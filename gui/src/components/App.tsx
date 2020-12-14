import React, { useEffect, useState } from 'react';
import { Button, Form, Input, Tabs } from 'antd';
import LineOptionsForm from './LineOptionsForm';
import { transformData } from '../util/parser';
import 'antd/dist/antd.css';

class InvalidServerEventError extends Error {
  constructor() {
    super('The server sent an invalid event over the wire.');
  }
}

type RowProps = {
  index: Number;
  line: String;
};

let Row = (props: RowProps) => {
  return (
    <tr>
      <td>
        {props.line}
      </td>
    </tr>
  );
};

type changeCommandFormValues = {
  command: String;
}

let changeCommand = (values: changeCommandFormValues) => {
  fetch('http://localhost:6846/api/command', {
    method: 'put',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(values)
  })
};

let ChangeCommandForm = () => {
  return (
    <>
      <Form layout="inline" onFinish={changeCommand}>
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
  const [initStatus, setInitStatus] = useState<'uninitialized' | 'pending' | 'initialized'>('uninitialized');

  useEffect(() => {
    if (initStatus !== 'uninitialized')
      return;

    setInitStatus('pending');

    fetch('http://localhost:6846/api/join', {
      method: 'post',
      headers: {
        'Content-Type': 'application/json'
      }
    }).finally(() => setInitStatus('initialized'));
  }, [initStatus, setInitStatus]);

  // Our default IFS is a newline character, but that can be changed at the user level.
  const [lineSeparators, setLineSeparators] = useState<string>('\\n');
  const [lineRegex, setLineRegex] = useState<string>('');
  const [lineIndices, setLineIndices] = useState<string>('');

  // Manages our current line buffer.
  const [stdout, setStdout] = useState<string | undefined>(undefined);
  // Allow for errors to be bubbled up.
  const [error, setError] = useState<Error | undefined>();

  // Listen for updates when the app is loaded (and cleanup after ourselves).
  useEffect(() => {
    const updateStream = new EventSource('http://localhost:6846/api/command/output');
    updateStream.onmessage = (event) => {
      if (!event?.data) {
        setError(new InvalidServerEventError());
        return
      }

      let data = JSON.parse(event.data);

      if (!data?.stdout) {
        setError(new InvalidServerEventError());
        return
      }

      setStdout(atob(data.stdout as string));
    };
    return () => updateStream.close();
  }, [setStdout]);

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
                      {transformData(lineSeparators, lineRegex, lineIndices, stdout || "").map((line, index) => (
                        <Row key={`${index}:${line}`} line={line} index={index} />
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
          <ChangeCommandForm />
          <LineOptionsForm separators={lineSeparators}
                           setSeparators={setLineSeparators}
                           setRegex={setLineRegex}
                           setIndices={setLineIndices} />
        </aside>
      </section>
    </>
  );
}

export default App;

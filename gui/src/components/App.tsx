import React, { useEffect, useState } from 'react';
import { Button, Form, Input, Tabs } from 'antd';
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

type changeInternalFieldSeparatorFormValues = {
  ifs: String;
}

let changeInternalFieldSeparator = (values: changeInternalFieldSeparatorFormValues) => {
  fetch('http://localhost:6846/api/internal-field-separator', {
    method: 'put',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(values)
  })
};

let ChangeInternalFieldSeparatorForm = () => {
  return (
    <>
      <Form layout="inline" onFinish={changeInternalFieldSeparator}>
        <Form.Item label="IFS" name="internal-field-separator">
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
  // Our default IFS is a newline character, but that can be changed at the user level.
  const [internalFieldSeparator, setInternalFieldSeparator] = useState<string>('\n');
  // Manages our current line buffer.
  const [stdout, setStdout] = useState<string | undefined>(undefined);
  // Allow for errors to be bubbled up.
  const [error, setError] = useState<Error | undefined>();

  // Listen for updates when the app is loaded (and cleanup after ourselves).
  useEffect(() => {
    const updateStream = new EventSource('http://localhost:6846/api/command/stdout');
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
        <main className="flex-grow">
          <Tabs defaultActiveKey="stdout">
            <Tabs.TabPane tab="stdout" key="stdout">
              {stdout ?
                <table className="font-mono">
                  <thead></thead>
                  <tbody>
                    {stdout?.split(internalFieldSeparator).map((line, index) => (
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
        </main>
        <aside className="w-1/4">
          <ChangeCommandForm />
          {/* TODO: Handle invalid input here. */}
          {/* TODO: Handle escaped characters. */}
          <ChangeInternalFieldSeparatorForm />
        </aside>
      </section>
    </>
  );
}

export default App;

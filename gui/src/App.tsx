import React, { useEffect, useState } from 'react';
import { Button, Form, Input, Tabs } from 'antd';

import 'antd/dist/antd.css';

const { TabPane } = Tabs;

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
    <tr key={`${props.index}:${props.line}`}>
      {props.line}
    </tr>
  );
};

type commandFormValues = {
  command: String;
}

let changeCommand = async (values: commandFormValues) => {
  await fetch('http://localhost:6846/api/command', {
    method: 'post',
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
        <Form.Item label="Command">
          <Input />
        </Form.Item>
        <Form.Item>
          <Button type="primary">Submit</Button>
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
      if (!event?.data || !(event.data instanceof String)) {
        setError(new InvalidServerEventError());
        return
      }

      setStdout(event.data as string);
    };
    return () => updateStream.close();
  });

  return (
    <>
      <input onChange={(event) => {
        // TODO: Handle invalid input here.
        // TODO: Handle escaped characters.
        if (event?.target?.value) {
          setInternalFieldSeparator(event.target.value);
        }
      }} />
      <ChangeCommandForm />
      <Tabs defaultActiveKey="stdout">
        <TabPane tab="stdout" key="stdout">
          {stdout ?
            <table>
              <thead></thead>
              <tbody>
                {stdout?.split(internalFieldSeparator).map((line, index) => (
                  <Row line={line} index={index} />
                ))}
              </tbody>
            </table>
            :
            <p>
              No data to show
            </p>
          }
        </TabPane>
        <TabPane tab="stderr" key="stderr">
        </TabPane>
      </Tabs>
    </>
  );
}

export default App;

import React, { useEffect, useState } from 'react';

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

let App = () => {
  // Our default IFS is a newline character, but that can be changed at the user level.
  const [internalFieldSeparator, setInternalFieldSeparator] = useState<string>('\n');
  // Manages our current line buffer.
  const [stdout, setStdout] = useState<string | undefined>(undefined);
  // Allow for errors to be bubbled up.
  const [error, setError] = useState<Error | undefined>();

  // Listen for updates when the app is loaded (and cleanup after ourselves).
  useEffect(() => {
    const updateStream = new EventSource('http://localhost:6846/api/stdout');
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
    <div className="container">
      <input onChange={(event) => {
        // TODO: Handle invalid input here.
        // TODO: Handle escaped characters.
        if (event?.target?.value) {
          setInternalFieldSeparator(event.target.value);
        }
      }} />
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
    </div>
  );
}

export default App;

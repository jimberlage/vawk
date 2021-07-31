import Papa from 'papaparse';
import * as React from 'react';
import * as ReactDOM from 'react-dom';
import { FromClient, FromServer, Initialize, SetRowSeparators, SetRowIndexFilters, SetRowRegexFilter, SetColumnSeparators, SetColumnIndexFilters, SetColumnRegexFilter } from './definitions_pb.js';
import "./app.css";

const SeparatorsOptions = ({ defaultSeparators, onChangeSeparators }) => (
  <div className='flex flex-col'>
    {defaultSeparators.map((separator, i) => (
      <div key={`${i}:${separator}`} className='flex flex-row'>
        <input
          defaultValue={separator}
          onBlur={(event) => {
            const newSeparators = [...defaultSeparators];
            newSeparators[i] = event.target.value;
            onChangeSeparators(newSeparators);
          }}
        />
        <button
          className='btn'
          onClick={(_event) => {
            var newSeparators = [...defaultSeparators];
            newSeparators = newSeparators.slice(0, i).concat(newSeparators.slice(i, -1));
            onChangeSeparators(newSeparators);
          }}
        >
          Delete
        </button>
      </div>
    ))}
    <button
      className='btn'
      onClick={(_event) => {
        var newSeparators = [...defaultSeparators];
        newSeparators.push('');
        onChangeSeparators(newSeparators);
      }}
    >
      Add
    </button>
  </div>
);

const Options = ({
  defaultSeparators,
  onChangeSeparators,
  defaultIndexFilters,
  onChangeIndexFilters,
  defaultRegexFilter,
  onChangeRegexFilter,
}) => (
  <div className='flex flex-col flex-1'>
    <SeparatorsOptions
      defaultSeparators={defaultSeparators}
      onChangeSeparators={onChangeSeparators}
    />
    <input
      type='text'
      defaultValue={defaultIndexFilters}
      onBlur={(event) => {
        onChangeIndexFilters(event.target.value);
      }}
    />
    <input
      type='text'
      defaultValue={defaultRegexFilter}
      onBlur={(event) => {
        onChangeRegexFilter(event.target.value);
      }}
    />
  </div>
);

const serializeSetRowSeparatorsMessage = (separators) => {
  const result = new FromClient();
  const message = new SetRowSeparators();
  message.setSeparatorsList(separators);
  result.setSetRowSeparators(message);
  return result.serializeBinary().buffer;
}

const serializeSetRowIndexFiltersMessage = (indexFilters) => {
  const result = new FromClient();
  const message = new SetRowIndexFilters();
  if (indexFilters !== '') {
    message.setFilters(indexFilters);
  }
  result.setSetRowIndexFilters(message);
  return result.serializeBinary().buffer;
}

const serializeSetRowRegexFilterMessage = (regexFilter) => {
  const result = new FromClient();
  const message = new SetRowRegexFilter();
  if (regexFilter !== '') {
    message.setFilter(regexFilter);
  }
  result.setSetRowRegexFilter(message);
  return result.serializeBinary().buffer;
}

const RowOptions = ({ connection, separators, setSeparators, indexFilters, setIndexFilters, regexFilter, setRegexFilter }) => {
  return (
    <Options
      defaultSeparators={separators}
      onChangeSeparators={(newSeparators) => {
        setSeparators(newSeparators);
        connection.send(serializeSetRowSeparatorsMessage(newSeparators));
      }}
      defaultIndexFilters={indexFilters}
      onChangeIndexFilters={(newIndexFilters) => {
        setIndexFilters(newIndexFilters);
        connection.send(serializeSetRowIndexFiltersMessage(newIndexFilters));
      }}
      defaultRegexFilter={regexFilter}
      onChangeRegexFilter={(newRegexFilter) => {
        setRegexFilter(newRegexFilter);
        connection.send(serializeSetRowRegexFilterMessage(newRegexFilter));
      }}
    />
  );
}

const serializeSetColumnSeparatorsMessage = (separators) => {
  const result = new FromClient();
  const message = new SetColumnSeparators();
  message.setSeparatorsList(separators);
  result.setSetColumnSeparators(message);
  return result.serializeBinary().buffer;
}

const serializeSetColumnIndexFiltersMessage = (indexFilters) => {
  const result = new FromClient();
  const message = new SetColumnIndexFilters();
  if (indexFilters !== '') {
    message.setFilters(indexFilters);
  }
  result.setSetColumnIndexFilters(message);
  return result.serializeBinary().buffer;
}

const serializeSetColumnRegexFilterMessage = (regexFilter) => {
  const result = new FromClient();
  const message = new SetColumnRegexFilter();
  if (regexFilter !== '') {
    message.setFilter(regexFilter);
  }
  result.setSetColumnRegexFilter(message);
  return result.serializeBinary().buffer;
}

const ColumnOptions = ({ connection, separators, setSeparators, indexFilters, setIndexFilters, regexFilter, setRegexFilter }) => {
  return (
    <Options
      defaultSeparators={separators}
      onChangeSeparators={(newSeparators) => {
        setSeparators(newSeparators);
        connection.send(serializeSetColumnSeparatorsMessage(newSeparators));
      }}
      defaultIndexFilters={indexFilters}
      onChangeIndexFilters={(newIndexFilters) => {
        setIndexFilters(newIndexFilters);
        connection.send(serializeSetColumnIndexFiltersMessage(newIndexFilters));
      }}
      defaultRegexFilter={regexFilter}
      onChangeRegexFilter={(newRegexFilter) => {
        setRegexFilter(newRegexFilter);
        connection.send(serializeSetColumnRegexFilterMessage(newRegexFilter));
      }}
    />
  );
}

const Sidebar = ({
  connection,
  defaultRowSeparators,
  defaultRowIndexFilters,
  defaultRowRegexFilter,
  defaultColumnSeparators,
  defaultColumnIndexFilters,
  defaultColumnRegexFilter
}) => {
  const [rowSeparators, setRowSeparators] = React.useState(defaultRowSeparators);
  const [rowIndexFilters, setRowIndexFilters] = React.useState(defaultRowIndexFilters);
  const [rowRegexFilter, setRowRegexFilter] = React.useState(defaultRowRegexFilter);
  const [columnSeparators, setColumnSeparators] = React.useState(defaultColumnSeparators);
  const [columnIndexFilters, setColumnIndexFilters] = React.useState(defaultColumnIndexFilters);
  const [columnRegexFilter, setColumnRegexFilter] = React.useState(defaultColumnRegexFilter);

  return (
    <div className='flex flex-col flex-1'>
      <div className='tabs'>
        <a className='tab tab-bordered'>Row</a>
        <a className='tab tab-bordered'>Column</a>
        <RowOptions
          connection={connection}
          separators={rowSeparators}
          setSeparators={setRowSeparators}
          indexFilters={rowIndexFilters}
          setIndexFilters={setRowIndexFilters}
          regexFilter={rowRegexFilter}
          setRegexFilter={setRowRegexFilter}
        />
        <ColumnOptions
          connection={connection}
          separators={columnSeparators}
          setSeparators={setColumnSeparators}
          indexFilters={columnIndexFilters}
          setIndexFilters={setColumnIndexFilters}
          regexFilter={columnRegexFilter}
          setRegexFilter={setColumnRegexFilter}
        />
      </div>
    </div>
  );
}

const Table = ({ rows }) => (
  <table className='table table-compact font-mono'>
    <tbody>
      {rows.map((row, i) => (
        <tr className='border-2 border-gray-600 border-solid' key={`${row.join()}:${i}`}>
          {row.map((cell, j) => (
            <td className='border-2 border-gray-600 border-solid' key={`${cell}:${j}`} style={{whiteSpace: 'pre'}}>
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
  console.log(messageEvent);
  const buffer = await messageEvent.data.arrayBuffer();
  const message = FromServer.deserializeBinary(buffer);
  console.log(message);
  if (message.hasUnexpectedError()) {
    throw new Error(message.getUnexpectedError().getDescription());
  }

  const stdout = bytesDecoder.decode(message.getOutput()).replace(/\n*$/, "");
  const parsedStdout = Papa.parse(stdout, {header: false, delimiter: ',', newline: '\n'});

  setRows(parsedStdout.data);
}

const serializeInitializeMessage = (rowSeparators, rowIndexFilters, rowRegexFilter, columnSeparators, columnIndexFilters, columnRegexFilter) => {
  const result = new FromClient();
  const message = new Initialize();
  message.setRowSeparatorsList(rowSeparators);
  if (rowIndexFilters !== '') {
    message.setRowIndexFilters(rowIndexFilters);
  }
  if (rowRegexFilter !== '') {
    message.setRowRegexFilter(rowRegexFilter);
  }
  message.setColumnSeparatorsList(columnSeparators);
  if (columnIndexFilters !== '') {
    message.setColumnIndexFilters(columnIndexFilters);
  }
  if (columnRegexFilter !== '') {
    message.setColumnRegexFilter(columnRegexFilter);
  }
  result.setInitialize(message);
  return result.serializeBinary().buffer;
}

const App = () => {
  const defaultRowSeparators = ['\\n'];
  const defaultRowIndexFilters = '';
  const defaultRowRegexFilter = '';
  const defaultColumnSeparators = ['\\s'];
  const defaultColumnIndexFilters = '';
  const defaultColumnRegexFilter = '';
  const [connection, setConnection] = React.useState();
  const [rows, setRows] = React.useState([]);

  // Set up the connection.
  React.useEffect(() => {
    if (!connection) {
      const newConnection = new WebSocket(`ws://${window.location.host}/ws/`);
      newConnection.onopen = (_event) => {
        setConnection(newConnection);
        newConnection.send(serializeInitializeMessage(defaultRowSeparators, defaultRowIndexFilters, defaultRowRegexFilter, defaultColumnSeparators, defaultColumnIndexFilters, defaultColumnRegexFilter));
      };
      newConnection.onclose = (_event) => {
        setConnection(undefined);
        window.close();
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
    <div className='flex flex-row flex-1'>
      <Table rows={rows} />
      <Sidebar
        connection={connection}
        defaultRowSeparators={defaultRowSeparators}
        defaultRowIndexFilters={defaultRowIndexFilters}
        defaultRowRegexFilter={defaultRowRegexFilter}
        defaultColumnSeparators={defaultColumnSeparators}
        defaultColumnIndexFilters={defaultColumnIndexFilters}
        defaultColumnRegexFilter={defaultColumnRegexFilter}
      />
    </div>
  );
};

ReactDOM.render(<App />, document.getElementById('app'));

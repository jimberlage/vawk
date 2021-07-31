import Papa from 'papaparse';
import * as React from 'react';
import * as ReactDOM from 'react-dom';
import { FromClient, FromServer, Initialize, SetRowSeparators, SetRowIndexFilters, SetRowRegexFilter, SetColumnSeparators, SetColumnIndexFilters, SetColumnRegexFilter } from './definitions_pb.js';
import "./app.css";

const BlurredInput = (props) => {
  const inputRef = React.useRef(null);

  return (
    <input
      onKeyPress={(event) => {
        if (props.onKeyPress) {
          props.onKeyPress(event);
        }

        if (event.key === 'Enter') {
          inputRef.current.blur();
        }
      }}
      ref={inputRef}
      {...props}
    />
  )
}

const SeparatorsOptions = ({ defaultSeparators, onChangeSeparators }) => (
  <div className='flex flex-col'>
    <label class='label'>
      <span class='label-text'>
        Add one or more separators to split on
      </span>
    </label>
    {defaultSeparators.map((separator, i) => (
      <div key={`${i}:${separator}`} className='flex flex-row items-center pb-4'>
        <div className='form-control flex-1'>
          <BlurredInput
            className='input input-bordered'
            type='text'
            defaultValue={separator}
            onBlur={(event) => {
              const newSeparators = [...defaultSeparators];
              newSeparators[i] = event.target.value;
              onChangeSeparators(newSeparators);
            }}
          />
        </div>
        <button
          className='btn btn-circle btn-xs flex justify-center items-center ml-2'
          onClick={(_event) => {
            var newSeparators = [...defaultSeparators];
            newSeparators = newSeparators.slice(0, i).concat(newSeparators.slice(i, -1));
            onChangeSeparators(newSeparators);
          }}
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
          </svg>
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
      Add separator
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
    <div className='form-control'>
      <label class='label'>
        <span class='label-text'>
          Add indices to keep here, like "3" or "0..9" or "5.."
        </span>
      </label>
      <BlurredInput
        className='input input-bordered'
        type='text'
        defaultValue={defaultIndexFilters}
        onBlur={(event) => {
          onChangeIndexFilters(event.target.value);
        }}
      />
    </div>
    <div className='form-control'>
      <label class='label'>
        <span class='label-text'>
          Add a regex that lines should match, like "\.gitignore"
        </span>
      </label>
      <BlurredInput
        className='input input-bordered'
        type='text'
        defaultValue={defaultRegexFilter}
        onBlur={(event) => {
          onChangeRegexFilter(event.target.value);
        }}
      />
    </div>
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
  const [isHidden, setIsHidden] = React.useState(false);
  const [tab, setTab] = React.useState('row');
  const [rowSeparators, setRowSeparators] = React.useState(defaultRowSeparators);
  const [rowIndexFilters, setRowIndexFilters] = React.useState(defaultRowIndexFilters);
  const [rowRegexFilter, setRowRegexFilter] = React.useState(defaultRowRegexFilter);
  const [columnSeparators, setColumnSeparators] = React.useState(defaultColumnSeparators);
  const [columnIndexFilters, setColumnIndexFilters] = React.useState(defaultColumnIndexFilters);
  const [columnRegexFilter, setColumnRegexFilter] = React.useState(defaultColumnRegexFilter);

  if (isHidden) {
    return (
      <button
        className='fixed top-4 right-4 btn btn-accent rounded-2xl opacity-60'
        onClick={(_event) => setIsHidden(false)}
      >
        Show controls
      </button>
    )
  }

  return (
    <div className='card text-center shadow-2xl flex h-screen-8 w-1/6 fixed top-4 right-4 bg-white'>
      <button
        className='btn btn-accent rounded-2xl mb-4'
        onClick={(_event) => setIsHidden(true)}
      >
        Hide
      </button>
      <div className='tabs w-full mb-4'>
        <a
          className={`tab tab-bordered ${tab === 'row' ? 'tab-active' : ''} flex-1`}
          onClick={(_event) => setTab('row')}
        >
          Row
        </a>
        <a
          className={`tab tab-bordered ${tab === 'column' ? 'tab-active' : ''} flex-1`}
          onClick={(_event) => setTab('column')}
        >
          Column
        </a>
      </div>
      <div className='px-2'>
        {tab === 'row' ? (
          <RowOptions
            connection={connection}
            separators={rowSeparators}
            setSeparators={setRowSeparators}
            indexFilters={rowIndexFilters}
            setIndexFilters={setRowIndexFilters}
            regexFilter={rowRegexFilter}
            setRegexFilter={setRowRegexFilter}
          />
        ) : (
          <ColumnOptions
            connection={connection}
            separators={columnSeparators}
            setSeparators={setColumnSeparators}
            indexFilters={columnIndexFilters}
            setIndexFilters={setColumnIndexFilters}
            regexFilter={columnRegexFilter}
            setRegexFilter={setColumnRegexFilter}
          />
        )}
      </div>
    </div>
  );
}

const TableRow = ({ row }) => {
  const [isHovered, setIsHovered] = React.useState(false);

  return (
    <tr
      className={isHovered ? 'active' : ''}
      onMouseEnter={(_event) => setIsHovered(true)}
      onMouseLeave={(_event) => setIsHovered(false)}
    >
      {row.map((cell, i) => (
        <td key={`${cell}:${i}`}>
          {cell}
        </td>
      ))}
    </tr>
  );
}

const Table = ({ rows }) => (
  <div className='flex flex-1 p-4'>
    {rows ? (
      <table className='table table-compact font-mono overflow-y-auto'>
        {rows.length > 0 ? (
          <thead>
            <tr>
              {rows[0].map((_, i) => (
                <th key={i}>{i}</th>
              ))}
            </tr>
          </thead>
        ) : null}
        <tbody>
          {rows.map((row, i) => (
            <TableRow key={`${row.join()}:${i}`} row={row} />
          ))}
        </tbody>
      </table>
    ) : null}
  </div>
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
  const [rows, setRows] = React.useState();

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
      <div className='flex w-8' />
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

import Papa from 'papaparse';
import copy from 'copy-to-clipboard';
import * as React from 'react';
import * as ReactDOM from 'react-dom';
import {
  And,
  Combination,
  FromClient,
  FromServer,
  Initialize,
  Or,
  SetColumnIndexFilters,
  SetColumnRegexSeparator,
  SetColumnSeparators,
  SetRowFilterCombination,
  SetRowIndexFilters,
  SetRowRegexFilter,
  SetRowRegexSeparator,
  SetRowSeparators,
} from './definitions_pb.js';
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

const SeparatorsOptions = ({
  defaultSeparators,
  onChangeSeparators,
  defaultSeparatorRegex,
  onChangeSeparatorRegex
}) => (
  <div className='flex flex-col flex-1'>
    <h1>Separators</h1>
    <label class='label'>
      <span class='label-text'>
        Add one or more separator literals to split on
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
    <label class='label'>
      <span class='label-text'>
        Or split on a regex
      </span>
    </label>
    <BlurredInput
      className='input input-bordered'
      type='text'
      defaultValue={defaultSeparatorRegex}
      onBlur={(event) => {
        onChangeSeparatorRegex(event.target.value);
      }}
    />
  </div>
);

const FiltersOptions = ({
  defaultIndexFilters,
  onChangeIndexFilters,
  filterCombination,
  onChangeFilterCombination,
  defaultRegexFilter,
  onChangeRegexFilter,
}) => (
  <div className='flex flex-col flex-1'>
    <h1>Filters</h1>
    <div className='form-control pb-4'>
      <label className='label'>
        <span className='label-text'>
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
    {defaultRegexFilter !== undefined && onChangeRegexFilter !== undefined ? (
      <>
        <div className='form-control items-center pb-4'>
          <div className='btn-group'>
            <input
              type='radio'
              data-title='And'
              className='btn'
              checked={filterCombination instanceof And}
              onChange={(_event) => {
                onChangeFilterCombination(new And());
              }}
            />
            <input
              type='radio'
              data-title='Or'
              className='btn'
              checked={filterCombination instanceof Or}
              onChange={(_event) => {
                onChangeFilterCombination(new Or());
              }}
            />
          </div>
        </div>
        <div className='form-control'>
          <label className='label'>
            <span className='label-text'>
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
      </>
    ) : null}
  </div>
);

const Options = ({
  defaultSeparators,
  onChangeSeparators,
  defaultSeparatorRegex,
  onChangeSeparatorRegex,
  filterCombination,
  onChangeFilterCombination,
  defaultIndexFilters,
  onChangeIndexFilters,
  defaultRegexFilter,
  onChangeRegexFilter,
}) => (
  <div className='flex flex-col flex-1'>
    <SeparatorsOptions
      defaultSeparators={defaultSeparators}
      onChangeSeparators={onChangeSeparators}
      defaultSeparatorRegex={defaultSeparatorRegex}
      onChangeSeparatorRegex={onChangeSeparatorRegex}
    />
    <div className='h-4' />
    <FiltersOptions
      defaultIndexFilters={defaultIndexFilters}
      onChangeIndexFilters={onChangeIndexFilters}
      filterCombination={filterCombination}
      onChangeFilterCombination={onChangeFilterCombination}
      defaultRegexFilter={defaultRegexFilter}
      onChangeRegexFilter={onChangeRegexFilter}
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

const serializeSetRowSeparatorRegexMessage = (separators) => {
  const result = new FromClient();
  const message = new SetRowRegexSeparator();
  message.setSeparator(separators);
  result.setSetRowRegexSeparator(message);
  return result.serializeBinary().buffer;
}

const serializeSetRowFilterCombinationMessage = (combination) => {
  const result = new FromClient();
  const message = new SetRowFilterCombination();
  const wrapper = new Combination();
  if (combination instanceof And) {
    wrapper.setAnd(combination);
  } else if (combination instanceof Or) {
    wrapper.setOr(combination);
  }
  message.setCombination(wrapper);
  result.setSetRowFilterCombination(message);
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

const RowOptions = ({
  connection,
  separators,
  setSeparators,
  separatorRegex,
  setSeparatorRegex,
  filterCombination,
  setFilterCombination,
  indexFilters,
  setIndexFilters,
  regexFilter,
  setRegexFilter,
}) => {
  return (
    <Options
      defaultSeparators={separators}
      onChangeSeparators={(newSeparators) => {
        setSeparators(newSeparators);
        connection.send(serializeSetRowSeparatorsMessage(newSeparators));
      }}
      defaultSeparatorRegex={separatorRegex}
      onChangeSeparatorRegex={(newSeparatorRegex) => {
        setSeparatorRegex(newSeparatorRegex);
        connection.send(serializeSetRowSeparatorRegexMessage(newSeparatorRegex));
      }}
      filterCombination={filterCombination}
      onChangeFilterCombination={(newFilterCombination) => {
        setFilterCombination(newFilterCombination);
        connection.send(serializeSetRowFilterCombinationMessage(newFilterCombination));
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

const serializeSetColumnSeparatorRegexMessage = (separators) => {
  const result = new FromClient();
  const message = new SetColumnRegexSeparator();
  message.setSeparator(separators);
  result.setSetColumnRegexSeparator(message);
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

const ColumnOptions = ({
  connection,
  separators,
  setSeparators,
  separatorRegex,
  setSeparatorRegex,
  indexFilters,
  setIndexFilters,
}) => {
  return (
    <Options
      defaultSeparators={separators}
      onChangeSeparators={(newSeparators) => {
        setSeparators(newSeparators);
        connection.send(serializeSetColumnSeparatorsMessage(newSeparators));
      }}
      defaultSeparatorRegex={separatorRegex}
      onChangeSeparatorRegex={(newSeparatorRegex) => {
        setSeparatorRegex(newSeparatorRegex);
        connection.send(serializeSetColumnSeparatorRegexMessage(newSeparatorRegex));
      }}
      defaultIndexFilters={indexFilters}
      onChangeIndexFilters={(newIndexFilters) => {
        setIndexFilters(newIndexFilters);
        connection.send(serializeSetColumnIndexFiltersMessage(newIndexFilters));
      }}
    />
  );
}

const Sidebar = ({
  connection,
  defaultRowSeparators,
  defaultRowSeparatorRegex,
  defaultRowFilterCombination,
  defaultRowIndexFilters,
  defaultRowRegexFilter,
  defaultColumnSeparators,
  defaultColumnSeparatorRegex,
  defaultColumnIndexFilters,
}) => {
  const [isHidden, setIsHidden] = React.useState(false);
  const [tab, setTab] = React.useState('row');
  const [rowSeparators, setRowSeparators] = React.useState(defaultRowSeparators);
  const [rowSeparatorRegex, setRowSeparatorRegex] = React.useState(defaultRowSeparatorRegex);
  const [rowFilterCombination, setRowFilterCombination] = React.useState(defaultRowFilterCombination);
  const [rowIndexFilters, setRowIndexFilters] = React.useState(defaultRowIndexFilters);
  const [rowRegexFilter, setRowRegexFilter] = React.useState(defaultRowRegexFilter);
  const [columnSeparators, setColumnSeparators] = React.useState(defaultColumnSeparators);
  const [columnSeparatorRegex, setColumnSeparatorRegex] = React.useState(defaultColumnSeparatorRegex);
  const [columnIndexFilters, setColumnIndexFilters] = React.useState(defaultColumnIndexFilters);

  if (isHidden) {
    return (
      <button
        className='fixed top-4 right-4 btn btn-accent rounded-2xl opacity-60 z-10'
        onClick={(_event) => setIsHidden(false)}
      >
        Show controls
      </button>
    )
  }

  return (
    <div className='card text-center shadow-2xl flex h-screen-8 w-1/6 fixed top-4 right-4 bg-white z-10'>
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
            separatorRegex={rowSeparatorRegex}
            setSeparatorRegex={setRowSeparatorRegex}
            filterCombination={rowFilterCombination}
            setFilterCombination={setRowFilterCombination}
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
            separatorRegex={columnSeparatorRegex}
            setSeparatorRegex={setColumnSeparatorRegex}
            indexFilters={columnIndexFilters}
            setIndexFilters={setColumnIndexFilters}
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
        <td key={`${cell}:${i}`} onClick={(_event) => copy(cell)}>
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

const serializeInitializeMessage = (rowSeparators, rowSeparatorRegex, rowFilterCombination, rowIndexFilters, rowRegexFilter, columnSeparators, columnSeparatorRegex, columnIndexFilters) => {
  const result = new FromClient();
  const message = new Initialize();
  message.setRowSeparatorsList(rowSeparators);
  if (rowSeparatorRegex !== '') {
    message.setRowRegexSeparator(rowSeparatorRegex)
  }

  const combination = new Combination();
  if (combination instanceof And) {
    combination.setAnd(rowFilterCombination);
  } else if (combination instanceof Or) {
    combination.setOr(rowFilterCombination);
  }
  message.setRowFilterCombination(combination);

  if (rowIndexFilters !== '') {
    message.setRowIndexFilters(rowIndexFilters);
  }
  if (rowRegexFilter !== '') {
    message.setRowRegexFilter(rowRegexFilter);
  }

  message.setColumnSeparatorsList(columnSeparators);
  if (columnSeparatorRegex !== '') {
    message.setColumnRegexSeparator(columnSeparatorRegex)
  }
  if (columnIndexFilters !== '') {
    message.setColumnIndexFilters(columnIndexFilters);
  }

  result.setInitialize(message);
  return result.serializeBinary().buffer;
}

const App = () => {
  const defaultRowSeparators = ['\\n'];
  const defaultRowSeparatorRegex = '';
  const defaultRowFilterCombination = new Or();
  const defaultRowIndexFilters = '';
  const defaultRowRegexFilter = '';
  const defaultColumnSeparators = [];
  const defaultColumnSeparatorRegex = '\\s+';
  const defaultColumnIndexFilters = '';
  const [connection, setConnection] = React.useState();
  const [rows, setRows] = React.useState();

  // Set up the connection.
  React.useEffect(() => {
    if (!connection) {
      const newConnection = new WebSocket(`ws://${window.location.host}/ws/`);
      newConnection.onopen = (_event) => {
        setConnection(newConnection);
        newConnection.send(serializeInitializeMessage(defaultRowSeparators, defaultRowSeparatorRegex, defaultRowFilterCombination, defaultRowIndexFilters, defaultRowRegexFilter, defaultColumnSeparators, defaultColumnSeparatorRegex, defaultColumnIndexFilters));
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
        defaultRowSeparatorRegex={defaultRowSeparatorRegex}
        defaultRowFilterCombination={defaultRowFilterCombination}
        defaultRowIndexFilters={defaultRowIndexFilters}
        defaultRowRegexFilter={defaultRowRegexFilter}
        defaultColumnSeparators={defaultColumnSeparators}
        defaultColumnSeparatorRegex={defaultColumnSeparatorRegex}
        defaultColumnIndexFilters={defaultColumnIndexFilters}
      />
    </div>
  );
};

ReactDOM.render(<App />, document.getElementById('app'));

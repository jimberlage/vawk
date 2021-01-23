interface ChunkMetadata {
    index: number,
    total: number,
    id: number,
};

export interface OutputMessage {
    chunks: string[],
    total: number,
    filled: number,
};

let addChunkToMessage = (metadata: ChunkMetadata, data: string, message: OutputMessage) => {
    if (metadata.index >= metadata.total || metadata.total !== message.total) {
        throw new Error("Got a malformed message from the server");
    }

    message.chunks[metadata.index] = data;
    message.filled += 1;
};

let allocateMessage = (metadata: ChunkMetadata, data: string): OutputMessage => {
    let chunks = Array(metadata.total);
    let message = {
        chunks,
        total: metadata.total,
        filled: 0,
    };

    if (metadata.total > 0) {
        addChunkToMessage(metadata, data, message);
    }

    return message;
};

let parseChunk = (chunk: string): [ChunkMetadata, string] => {
    let parts = chunk.split('\n', 2);
    if (parts.length !== 2) {
        throw new Error("Got a malformed message from the server");
    }
    let metadata = JSON.parse(parts[0]) as ChunkMetadata;
    return [metadata, parts[1]];
};

export let addChunk = (chunk: string, message: OutputMessage | undefined): OutputMessage => {
    let [metadata, data] = parseChunk(chunk);
    if (message) {
        addChunkToMessage(metadata, data, message);
        return message;
    } else {
        return allocateMessage(metadata, data);
    }
};

export let isComplete = (message: OutputMessage): boolean => {
    return message.filled === message.total;
};

export let combineStdoutChunks = (message: OutputMessage): string[][] => {
    let encodedTable = JSON.parse(message.chunks.join('')) as string[][];
    return encodedTable.map((encodedRow) => encodedRow.map((encodedCell) => atob(encodedCell)));
};

export let combineStderrChunks = (message: OutputMessage): string => {
    return atob(message.chunks.join(''));
};

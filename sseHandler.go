package main

import (
	"context"
	"net/http"
	"sync/atomic"

	"github.com/tessellator/fnrun"
)

type sseHandler struct {
	lastClientID *uint64
	outboxes     map[uint64]chan []byte
}

func (h *sseHandler) assignClientID() uint64 {
	id := atomic.AddUint64(h.lastClientID, 1)
	h.outboxes[id] = make(chan []byte, 1024)
	return id
}

func (h *sseHandler) ServeHTTP(rw http.ResponseWriter, req *http.Request) {
	flusher, ok := rw.(http.Flusher)
	if !ok {
		http.Error(rw, "This client does not support server-side streaming", http.StatusPreconditionFailed)
		return
	}

	// Ensure that CORS support works so that we can send requests from file:// URLs or localhost.
	rw.Header().Set("Access-Control-Allow-Origin", "*")
	rw.Header().Set("Content-Type", "text/event-stream")
	rw.Header().Set("Cache-Control", "no-cache")
	rw.Header().Set("Connection", "keep-alive")

	clientID := h.assignClientID()

	for {
		outbox := <-h.outboxes[clientID]
		n, err := rw.Write(outbox)

		if err != nil {
			http.Error(rw, "Failed to read some input", http.StatusInternalServerError)
			return
		}

		if n != len(outbox) {
			http.Error(rw, "Failed to read the whole input", http.StatusInternalServerError)
			return
		}

		flusher.Flush()
	}
}

var handler *sseHandler = nil

// onSetup ensures that the handler is defined and has bound to a port.
func onSetup() {
	if handler != nil {
		return
	}

	lastClientID := uint64(0)
	handler = &sseHandler{
		lastClientID: &lastClientID,
		outboxes:     map[uint64]chan []byte{},
	}

	go func() {
		err := http.ListenAndServe(":9898", handler)
		if err != nil {
			// If we fail to bind to a port here, there's nothing the library can do.
			panic(err)
		}
	}()
}

// Sink provides the entrypoint to run the server and send server-side-events to connected browsers.
func Sink(ctx context.Context, result *fnrun.Result) error {
	// Setup a singleton server, if none exists.
	onSetup()

	// Send the message to every connected client.
	for _, outbox := range handler.outboxes {
		outbox <- result.Data
	}

	return nil
}

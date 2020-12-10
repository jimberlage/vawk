package main

import (
	"context"
	"time"

	"github.com/tessellator/fnrun"
)

func Source(ctx context.Context, invoker fnrun.Invoker) error {
	ticker := time.NewTicker(5 * time.Second)
	for {
		select {
		case <-ticker.C:
			invoker.Invoke(ctx, &fnrun.Input{Data: []byte(time.Now().String())})
		}
	}

	return nil
}

---
keywords:
- documentation
- inx
- reference
- sync
---

# INX Syncing

The primary function of Chronicle is to gather tangle data via an INX connection. This connection will most likely be to an IOTA Node, which should be on the same machine as the Chronicle instance.

When running with the `inx` feature (default using the docker image), Chronicle will stream data from the `INX_URL` and store it in the configured `MongoDB` instance.

## Ledger State

When Chronicle starts syncing, it will get the current Ledger State from the INX source. Though Chronicle can sync back to the earliest data the INX connection can provide, the data may not be valid until it catches up to the ledger index of that initial state.

## Analytics

The starting index determines when Analytics can begin. Analytics are calculated incrementally per-milestone, but require an initial ledger state which is only valid once Chronicle is caught up to the starting index.

# CLI Analytics

Analytics can also be synced manually using one of two command-line arguments: `fill-analytics` and `fill-interval-analytics`. Neither should be run until Chronicle reports that it is healthy (see the `/health` endpoint) or else the results may not be correct. However, if this happens, re-running the analytics when Chronicle is healthy will overwrite the previous values.

The `--num-tasks` flag can be used to parallelize these processes, but be warned: There is currently no feature to resume halted analytics. If the process is canceled without completing, it may be very difficult to restart it without re-calculating analytics.

For the list of supported analytics of each kind and more information, refer to the CLI documentation (via `chronicle --help`).

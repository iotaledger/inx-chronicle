# inx-chronicle

[![ci](https://github.com/iotaledger/inx-chronicle/actions/workflows/ci.yml/badge.svg)](https://github.com/iotaledger/inx-chronicle/actions/workflows/ci.yml)
[![Canary](https://github.com/iotaledger/inx-chronicle/actions/workflows/canary.yml/badge.svg)](https://github.com/iotaledger/inx-chronicle/actions/workflows/canary.yml)
[![Coverage Status](https://coveralls.io/repos/github/iotaledger/inx-chronicle/badge.svg?branch=main)](https://coveralls.io/github/iotaledger/inx-chronicle?branch=main)

Chronicle is the permanode (sometimes also called indexer or scanner) for the IOTA-based networks.
It connects to a [Hornet](https://github.com/iotaledger/hornet) via the [IOTA Node Extension (INX)](https://github.com/iotaledger/inx) interface.
Through the INX interface, Chronicle listens to all blocks in the Tangle that are referenced by a milestone and stores them in a [MongoDB](https://www.mongodb.com/) database.

Chronicle offers several APIs that are documented [here](https://wiki.iota.org/inx-chronicle/reference/api.md).

## Documentation

The documentation for Chronicle can be found in the IOTA wiki: https://wiki.iota.org/chronicle/welcome

## Development

This project uses [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) to automatically generate a [`CHANGELOG.md`] which signals breaking changes.
The changelog can be created using the following command (requires the [`conventional-changelog-cli`](https://www.npmjs.com/package/conventional-changelog-cli) package on NPM):
```sh
conventional-changelog -p conventionalcommits -i CHANGELOG.md -s
```

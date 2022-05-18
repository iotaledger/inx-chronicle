# inx-chronicle

[![CI](https://github.com/iotaledger/inx-chronicle/actions/workflows/ci.yml/badge.svg)](https://github.com/iotaledger/inx-chronicle/actions/workflows/ci.yml)
[![Canary](https://github.com/iotaledger/inx-chronicle/actions/workflows/canary.yml/badge.svg)](https://github.com/iotaledger/inx-chronicle/actions/workflows/canary.yml)
[![Coverage Badge](https://coveralls.io/repos/github/iotaledger/inx-chronicle/badge.svg?branch=main "Coverage Badge")](https://coveralls.io/github/grtlr/inx-chronicle)

## Usage

The easiest way to start Chronicle is by using our supplied Dockerfile.

First you need to setup the correct permissions for the node's database:

```sh
mkdir docker/alphanet

groupadd -g 65532 nonroot
useradd -g nonroot -u 65532 nonroot
chown nonroot:nonroot docker/alphanet/
```

After that, with Docker installed on your system, you can spin up Chronicle by running the following command from the root of the repository.

```sh
docker-compose -f docker/docker-compose.hornet.yml up
```

## Development

The easiest way to get going is to use a `private_tangle` for now:
```sh
git clone git@github.com:gohornet/hornet.git
cd hornet/private_tangle
./bootstrap.sh # exit with <CTRL-C>
./run.sh
```

Then you should be able to connect to INX on `http://localhost:9029`

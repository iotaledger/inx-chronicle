# inx-chronicle

[![ci](https://github.com/iotaledger/inx-chronicle/actions/workflows/ci.yml/badge.svg)](https://github.com/iotaledger/inx-chronicle/actions/workflows/ci.yml)
[![Canary](https://github.com/iotaledger/inx-chronicle/actions/workflows/canary.yml/badge.svg)](https://github.com/iotaledger/inx-chronicle/actions/workflows/canary.yml)
[![Coverage Status](https://coveralls.io/repos/github/iotaledger/inx-chronicle/badge.svg?branch=main)](https://coveralls.io/github/iotaledger/inx-chronicle?branch=main)

## APIs

The data within Chronicle can be accessed through the following APIs:

* [Core Node API](https://editor.swagger.io/?url=https://raw.githubusercontent.com/iotaledger/tips/stardust-api/tips/TIP-0025/core-rest-api.yaml) `api/core/v2/…`
* [Explorer API](https://editor.swagger.io/?url=https://raw.githubusercontent.com/iotaledger/inx-chronicle/main/docs/api-explorer.yml) `api/explorer/v2/…`
* [Analytics API](https://editor.swagger.io/?url=https://raw.githubusercontent.com/iotaledger/inx-chronicle/main/docs/api-analytics.yml) `api/analytics/v2/…`

## Usage

The easiest way to start Chronicle is by using our supplied Dockerfile.

First you need to setup the correct permissions for the node's database:

```sh
mkdir docker/hornet_data
groupadd -g 65532 nonroot
useradd -g nonroot -u 65532 nonroot
chown nonroot:nonroot docker/hornet_data/
```

We mount the MongoDB database as an additional volume, with appropriate permissions:
```sh
mkdir docker/chronicle_data
chown 999:999 docker/chronicle_data/
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

## JWT Authentication

Usage of the Chronicle API can be protected using [JWT](https://jwt.io/), by setting the following configuration settings under the `api` table in [config.toml](bin/inx-chronicle/config.template.toml).

- `password_hash` - The [argon2i](https://argon2.online/) hash of your chosen password.
- `password_salt` - The salt used to hash the above password.
- `public_routes` - A list of routes that can be accessed without providing a token. These can include the wildcard (*) symbol to allow any sequence of characters to match.

All JWT interactions should be performed via HTTPS.

### Public Routes

When a route is configured to be public, it can be accessed freely without providing a JWT. Thus, you should take care when specifying these routes, as a mis-configured route can open the application up to attacks. The only accepted special character is the wildcard (`*`), which will be converted to a regex `.*` and match against the original URI.

For instance, a request `GET https://localhost:XXXX/api/core/v2/milestones/by-index/10000` will check the set of public routes against the segment `/api/core/v2/milestones/by-index/10000`. 

Matching strings include:

- `/api/*`
- `/api/core/*/milestones/by-index/*`
- `*10000`

Non-matching strings include:

- `/core/v2/milestones/by-index/*`
- `/api/core/v2/milestones/by-index`
- `/api/core/v1/*`

If JWT is used, these routes should be as specific as possible to avoid accidentally exposing unintended routes.

### Keys

Chronicle uses an EdDSA secret key to create tokens, which can be generated by the application at startup or provided as an identity file using the `identity_path` config. Currently, this file must be a PKCS8 secret key ([RFC 5208](https://datatracker.ietf.org/doc/html/rfc5208)) PEM file. The location of this file can also optionally be specified using the `IDENTITY_PATH` env variable, which will be overridden by the config file value. If no such file is provided, a secret key is randomly generated for use while the application is running.

### Generating a Token

A special route at the root (`/login`) is provided for generating a new token. This token will use the password config as well as the `jwt_expiration` and the secret key. This token can be manually generated by the client, if desired, by using the same identity and claims.

Static claims used by Chronicle are:

- `iss`: `"chronicle"`
- `aud`: `"api"`

The `sub` (subject) claim is filled using a unique UUID, however it is not currently stored or validated by Chronicle.

### Providing a Token

To provide a token when making a request, include it in an `Authorization` header using the `Bearer` authentication scheme.

### Environment Variables

Currently Chronicle supports the following environment variables:

CONFIG_PATH="<FILE_PATH>": sets the file path to the `config.toml` file;
INX=<true|false>: enables/disables INX;
API=<true|false>: enables/disables the REST API;
METRICS=<true|false>: enables/disables the Metrics Server;

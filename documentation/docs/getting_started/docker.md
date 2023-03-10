---
keywords:
- documentation
- docker
- guide
---

# Run `INX Chronicle` using Docker

This guide describes the necessary steps to set up and run `Chronicle` as an INX plugin of a `Hornet` node to persist Tangle data, gather time series analytics using `InfluxDB`, and display it in meaningful ways using `Grafana`.

## Prerequisites

1. A recent release of Docker enterprise or community edition. You can find installation instructions in the [official Docker documentation](https://docs.docker.com/engine/install/).
2. [Docker Compose V2](https://docs.docker.com/compose/install/).

## Preparation

Create the necessary directories for all `Hornet` and `Chronicle` databases:

```
./docker/create_dirs.sh
```

## Configuration

Configure the `hornet` docker image via command-line arguments by editing the `docker-compose.yml` file. See the [Hornet Wiki](https://wiki.iota.org/hornet/references/configuration/) for details.

## Docker Image Build Variants

Chronicle has two build variants, which can be selected using the corresponding YML override file.

### Production

```sh
docker compose -f docker/docker-compose.yml -f docker/docker-compose.prod.yml up
```

### Debug


```sh
docker compose -f docker/docker-compose.yml up
```

## Analytics and Metrics

To run the images needed to support the Metrics and Analytics dashboards, run `docker compose` using the `metrics` profile:

```sh
docker compose -f docker/docker-compose.yml --profile metrics up
```

Access the Grafana dashboard at `http://localhost:3000/`.

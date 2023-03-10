---
keywords:
- documentation
- docker
- guide
---

# Run `Chronicle` (as an INX plugin of `Hornet`) using Docker

This guide describes the necessary steps to set up and run `Chronicle` as an INX plugin of a `Hornet` node to persist Tangle data without pruning (make sure to maintain sufficient hard disk storage), gather time series analytics data from the Tangle and the UTXO ledger, and display such data in meaningful ways using Grafana. 

## Prerequisites

1. A recent release of Docker enterprise or community edition. You can find installation instructions in the [official Docker documentation](https://docs.docker.com/engine/install/).
2. [Docker Compose CLI plugin](https://docs.docker.com/compose/install/compose-plugin/).
3. A local clone of the [Chronicle](https://github.com/iotaledger/inx-chronicle.git) repository

Enter the `docker` folder:

```sh
cd inx-chronicle/docker
```

## Preparations 

Create the necessary directories for all `Hornet` and `Chronicle` databases:

```sh
sudo ./create_dirs.sh
```

## Configuration

Configure `Hornet` through command-line arguments by editing the `docker-compose.yml` file

```sh
<your-favorite-text-editor> docker-compose.yml
```

Head to `Hornet`s command-line argument section:

```yml
hornet:
    command:
```

### Network (optional) 

Choose a network (default=Shimmer) to participate in by uncommenting one of the `config*.json` lines as shown below:

```yml
- "-c"
# - "config_testnet.json"
# - "config_alphanet.json"
```
### Peers (mandatory)

The most convenient way to get peers is to use autopeering. As the name suggests, Hornet will try to find its peers automatically and learn about more and more nodes in the particular network, and switch between them regularly. 

```yml
- "--p2p.autopeering.enabled=true"
```
However, this also reveals the existence of your node to everyone else in the network, which you may not want to. In that case, you have to use manual peering. Instead of the above you would then have to add those lines:

```yml
- "--p2p.peerAliases 'alice','bob'"
- "--p2p.peers '<multiaddr_1>','<multiaddr_2>'"
```

Make sure to keep a 1:1 relationship between the aliases and the peer's addresses. See also the [Hornet Wiki](https://wiki.iota.org/hornet/references/configuration/) for further details.

 A Hornet peering `Multiaddr` has the following format:
```
/ip4/198.51.100.0/tcp/15600/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N
```

Once you've configured peering, save and close the editor.

## Run containers

### Production Build

Chronicle will perform best if you run it in production mode as the build will be maximally optimized. However, this comes at the cost of longer compilation times.

```sh
docker compose -f docker-compose.prod.yml up -d
```

If you want to generate live metrics and run a Grafana server to monitor what's going on in the network, run instead:

```sh
docker compose -f docker-compose.prod.yml --profile metrics up -d
```

Both of those will run all containers as services detached from the current terminal session, which keeps them running until you explicitly stop them, but the latter will also generate all live metrics and spin up the Grafana server to monitor them.

### Debug Build

The simplest way of running `Hornet` with `Chronicle` is to run:

```sh
docker compose up -d
```

This will use the `docker-compose.yml` (without `.prod`) and run `Chronicle` in debug mode building it using the `Dockerfile.debug` file, and also without any metrics or Tangle data analytics.

Of course, you can enable `metrics` here as well:

```sh
docker compose --profile metrics up -d
```

## 4. Access Grafana dashboard

You can now access the Grafana dashboard at `http://<IP>:3000/`. Be aware however, that data might not show up immediatedly. `Chronicle` needs to fetch the full ledger state from `Hornet` first before it can start producing live analytics.

Note: The dashbaord is only available if you ran `docker compose` with the `--profile "metrics"` option.

Note: Some analytics cannot correctly be produced live on a per-milestone basis (the time interval analytics like `daily addresses`), and hence, some charts may stay empty until you run the appropriate CLI command to fill them (`--fill-interval-analytics`).





---
keywords:
- documentation
- docker
---

# Setting up `Hornet` with `Chronicle` as INX plugin for permanent data storage and Tangle and Ledger live analytics

This is a guide to set up a `Hornet` node together with `Chronicle` as an INX plugin to persist Tangle data without pruning (make sure to maintain sufficient hard disk storage), gather time series analytics data from the Tangle and the UTXO ledger, and display such data in meaningful ways using Grafana. 

Once you've cloned the repository from [Github](https://github.com/iotaledger/inx-chronicle.git), switch to the `docker` folder:
```sh
cd inx-chronicle/docker
```

## 1. Create the necessary directories for all `Hornet` and `Chronicle` databases

```sh
sudo ./create_dirs.sh
```

## 2. Configure `Hornet` by editing the `docker-compose.yml` file

```sh
<your-favorite-text-editor> docker-compose.yml
```

### Head to `Hornet`s CLI section:

```yml
hornet:
    command:
```

### (Optional) Select another network (default=Shimmer) by uncommenting one of the `config*.json` lines as shown below:

```yml
- "-c"
# - "config_testnet.json"
# - "config_alphanet.json"
```
### Configure peers for Hornet:

The most convenient way is to use autopeering. Hornet will try to find its peers automatically and learn about more and more nodes in the particular network, and switch between them regularly. 

```yml
- "--p2p.autopeering.enabled=true"
```
However, this also reveals the existence of your node to everyone else in the network, which you may not want to. In that case, you should set up manual peering only (but you can also use it additionally). This is how you set it up:

```yml
--p2p.peerAliases "alice","bob"
--p2p.peers "<multiaddr_1>","<multiaddr_2>"
```

Note, that `alice` belongs to `multiaddr_1` and `bob` belongs to `multiaddr_2`. There is always a 1:1 relationship between the alias and the multiaddress, and the same order must be kept for Hornet to associate them correctly. If you don't know how a multiaddr looks like here's an example based off of [libp2p](https://docs.libp2p.io/concepts/fundamentals/addressing/).
```
/ip4/198.51.100.0/tcp/15600/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N
```


Once you've set up peering, save and close the editor. There's only one final step to do.

## 3. Run all containers

### Running In Production

Chronicle will perform best if you run it in production mode as the build will be maximally optimized. However, this comes at the cost of longer compilation times.

```sh
docker compose -f "docker-compose.prod.yml" up -d
```

If you want to generate live metrics and run a Grafana server to monitor what's going on in the network, run instead:

```sh
docker compose -f "docker-compose.prod.yml" --profile "metrics" up -d
```

Both of those will run all containers as services detached from the current terminal session, which keeps them running until you explicitly stop some or all containers with

```sh
docker compose stop
```

but the latter will also generate all live metrics and spin up the Grafana server to monitor them.

If you are interested in certain logging events by any of the containers, e.g. emitted by Chronicle, you can simply follow along with running:

```sh
docker compose logs -f inx-chronicle
```

### Running In Debug

The simplest way of running `Hornet` with `Chronicle` is to run:

```sh
docker compose up -d
```

This will use the `docker-compose.yml` (without `.prod`) and run `Chronicle` in debug mode building it using the `Dockerfile.debug` file, and also without any metrics or Tangle data analytics.

Of course, you can enable `metrics` here as well:

```sh
docker compose --profile "metrics" up -d
```

## 4. Access Grafana dashboard

You can now access the Grafana dashboard at `http://<IP>:3000/`. Be aware however, that data might not show up immediatedly. `Chronicle` needs to fetch the full ledger state from `Hornet` first before it can start producing live analytics.

Note: The dashbaord is only available if you ran `docker compose` with the `--profile "metrics"` option.

Note: Some analytics cannot correctly be produced live on a per-milestone basis (the time interval analytics like `daily addresses`), and hence, some charts may stay empty until you run the appropriate CLI command to fill them (`--fill-interval-analytics`).





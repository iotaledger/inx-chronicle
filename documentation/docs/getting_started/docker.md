---
keywords:
- documentation
- docker
---

# Docker

## Usage

The easiest way to start Chronicle is by using our supplied Dockerfile.

First you need to setup the correct permissions for the node's database:

```sh
./docker/prepare_docker.sh
```

After that, with Docker installed on your system, you can spin up Chronicle by running the following command from the root of the repository.

```sh
docker-compose -f docker/docker-compose.yml up --build
```

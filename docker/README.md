## Development

To update to a new version of Hornet, simply adjust the image in `docker-compose.hornet.yml`.
We also bundle a minimal Hornet config in `config.*.json`.
This file can be updated by running the following from the project root:

```sh
./docker/update_hornet_config.sh
```

This script fetches the default Hornet configuration and applies the `hornet_config.patch` patch to enable INX.

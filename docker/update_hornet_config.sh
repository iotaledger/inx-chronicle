#! /bin/bash

DIR=$(dirname ${BASH_SOURCE[0]})
wget https://raw.githubusercontent.com/gohornet/hornet/develop/config_alphanet.json -O ${DIR}/config.alphanet.hornet.json

# We apply a patch to enable INX
git apply docker/hornet_config.patch

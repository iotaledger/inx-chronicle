#! /bin/bash

DIR=$(dirname ${BASH_SOURCE[0]})
wget https://raw.githubusercontent.com/gohornet/hornet/develop/config_alphanet.json -O ${DIR}/config.alphanet.hornet.json
wget https://raw.githubusercontent.com/gohornet/hornet/develop/config_testnet.json -O ${DIR}/config.testnet.hornet.json

#! /bin/bash

DIR=$(dirname ${BASH_SOURCE[0]})
wget https://raw.githubusercontent.com/gohornet/hornet/develop/config.json -O ${DIR}/config.hornet.json

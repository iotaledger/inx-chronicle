#!/bin/bash

DIR=$(dirname ${BASH_SOURCE[0]})

if [[ "$OSTYPE" != "darwin"* && "$EUID" -ne 0 ]]; then
  echo "Please run as root or with sudo"
  exit
fi

# Prepare Hornet's data directory.
mkdir -p ${DIR}/data/hornet
mkdir -p ${DIR}/data/chronicle
mkdir -p ${DIR}/data/chronicle/mongo1
mkdir -p ${DIR}/data/chronicle/mongo2
mkdir -p ${DIR}/data/chronicle/mongo3
mkdir -p ${DIR}/data/grafana
mkdir -p ${DIR}/data/prometheus
if [[ "$OSTYPE" != "darwin"* ]]; then
  chown -R 65532:65532 ${DIR}/data
fi

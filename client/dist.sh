#!/usr/bin/env bash

set -euxo pipefail

WS_URL=wss://go.kahv.io/ws/ dx build --release
docker build --tag seequo/vgs-client --platform linux/amd64 -f Dockerfile .
docker push seequo/vgs-client

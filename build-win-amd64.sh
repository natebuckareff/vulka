#!/usr/bin/env bash
docker build -f ./dockerfile.win-amd64 --progress=plain .

id=$(docker create $(docker build -f ./dockerfile.win-amd64 -q .))
docker cp $id:/rust/build/vulka-win-amd64.zip .
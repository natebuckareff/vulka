#!/usr/bin/env bash
docker build -f ./dockerfile.win-x86_64 --progress=plain .

id=$(docker create $(docker build -f ./dockerfile.win-x86_64 -q .))
docker cp $id:/rust/build/vulka-win-x86_64.zip .
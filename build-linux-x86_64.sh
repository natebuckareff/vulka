#!/usr/bin/env bash
docker build -f ./dockerfile.linux-x86_64 --progress=plain .

id=$(docker create $(docker build -f ./dockerfile.linux-x86_64 -q .))
docker cp $id:/rust/build/vulka-linux-x86_64.zip .
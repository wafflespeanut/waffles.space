#!/usr/bin/bash

ALIAS_ADDR="10.0.0.10"

# `localhost` and `0.0.0.0` conflicts with docker containers. It's better
# to alias the IP address (that's what Kubernetes does).
sudo ifconfig eth0:1 ${ALIAS_ADDR} netmask 255.255.255.0 up

echo 'Launching static file server...'
docker run --name static \
    --restart always \
    -v ~/source:/source \
    -v ~/private:/private \
    -p ${ALIAS_ADDR}:8000:8000 \
    -e CUSTOM_4XX=/source/4xx.html \
    -e SOURCE=/source \
    -e PRIVATE_SOURCE=/private \
    -e CONFIG=/config.json -d \
    wafflespeanut/static-server

echo 'Deploying Nginx proxy...'
docker run --name nginx \
    --restart always \
    -v ~/nginx/nginx.conf:/etc/nginx/nginx.conf \
    -v ~/nginx/default.conf:/etc/nginx/conf.d/default.conf \
    -p 80:80 -d \
    nginx:1.13-alpine

echo 'Launching ASCII art generator...'
docker run --name ascii-gen \
    --restart always \
    -p ${ALIAS_ADDR}:5000:5000 -d \
    wafflespeanut/ascii-gen

if [[ $(ps aux | grep watch.sh | wc -l) < 2 ]]; then
    # This only has the `grep` command
    echo 'Starting watcher...'
    nohup ./watch.sh &
fi

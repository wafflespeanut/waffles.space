#!/usr/bin/bash

docker network create waffles

while read line; do eval "export $line"; done < /home/core/server_env

echo 'Launching static file server...'
docker run --name static \
    --restart always \
    --network waffles \
    -v /home/core/source:/source \
    -v /home/core/private:/private \
    -v /home/core/config:/config \
    -e CUSTOM_4XX=/source/4xx.html \
    -e SOURCE=/source \
    -e PRIVATE_SOURCE=/private \
    -e CONFIG=/config/static_server_config.json -d \
    wafflespeanut/static-server

echo 'Launching ASCII art generator...'
docker run --name ascii-gen \
    --restart always \
    --network waffles \
    -d wafflespeanut/ascii-gen

echo 'Deploying Nginx proxy...'
docker run --name nginx \
    --restart always \
    --network waffles \
    -v /home/core/config/nginx.conf:/etc/nginx/nginx.conf \
    -v /home/core/config/default.conf:/etc/nginx/conf.d/default.conf \
    -v /home/core/letsencrypt/live/waffles.space:/etc/certs \
    -v /home/core/letsencrypt/archive:/archive \
    -p 80:80 -p 443:443 -d nginx:alpine

echo 'Deploying callbacks listener'
docker run --name callbacks \
    --restart always \
    --network waffles \
    -e SECRET=${CALLBACK_SECRET} \
    -e SKYPE_USER=${SKYPE_USER} -e SKYPE_PASS=${SKYPE_PASS} \
    -d wafflespeanut/server-callbacks

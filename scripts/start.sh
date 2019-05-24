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
    -e SOURCE=/source \
    -e PRIVATE_SOURCE=/private \
    -e CONFIG=/config/static_server_config.json -d \
    -e LOG_LEVEL=info \
    -e SMS_RECEIVER=${SMS_RECEIVER} \
    -e AWS_REGION=${AWS_REGION} \
    -e AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} \
    -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
    -e TWILIO_ACCOUNT=${TWILIO_ACCOUNT} \
    -e TWILIO_TOKEN=${TWILIO_TOKEN} \
    -e TWILIO_SENDER=${TWILIO_SENDER} \
    wafflespeanut/static-server

echo 'Deploying Nginx proxy...'
docker run --name nginx \
    --restart always \
    --network waffles \
    -v /home/core/config/nginx.conf:/etc/nginx/nginx.conf \
    -v /home/core/config/default.conf:/etc/nginx/conf.d/default.conf \
    -v /home/core/letsencrypt/live/waffles.space:/etc/certs \
    -v /home/core/letsencrypt/archive:/archive \
    -p 80:80 -p 443:443 -d nginx:alpine

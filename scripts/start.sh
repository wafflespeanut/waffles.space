#/bin/bash

docker network create waffles

while read line; do eval "export $line"; done < /root/server_env

echo 'Launching static file server...'
docker pull wafflespeanut/static-server
docker run --name static \
    --restart always \
    --network waffles \
    -v /root/source:/source \
    -v /root/private:/private \
    -v /root/config:/config \
    -e SOURCE=/source \
    -e PRIVATE_SOURCE=/private \
    -e CONFIG=/config/static_server_config.json \
    -e LOG_LEVEL=info \
    -d wafflespeanut/static-server

echo 'Launching ace game!'
docker pull wafflespeanut/ace-away
docker run --name ace-away \
    --cpus="0.25" \
    --memory="128m" \
    --restart always \
    --network waffles \
    -d wafflespeanut/ace-away

# echo 'Launching onchain scanner...'
# docker pull wafflespeanut/onchain-scanner
# docker run --name scanner \
#     --restart always \
#     --network waffles \
#     -v /root/config:/config \
#     -e ADDR=localhost:3001 \
#     -e AUTH_KEY=${SCANNER_AUTH} \
#     -e AWS_ACCESS_KEY=${AWS_ACCESS_KEY=} \
#     -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
#     -e CONFIG=/config/scanner-config.json \
#     -d wafflespeanut/onchain-scanner

echo 'Deploying Nginx proxy...'
docker run --name nginx \
    --restart always \
    --network waffles \
    -v /root/config/nginx.conf:/etc/nginx/nginx.conf \
    -v /root/config/default.conf:/etc/nginx/conf.d/default.conf \
    -v /root/self_signed:/etc/certs \
    -v /root/letsencrypt/archive:/archive \
    -p 80:80 -p 443:443 \
    -d nginx:alpine

#!/usr/bin/bash

echo 'Removing existing images and containers...'
docker rm -f $(docker ps -a -q);
docker rmi $(docker images -a -q);

./start.sh

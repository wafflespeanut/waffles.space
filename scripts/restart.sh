#/bin/bash

echo 'Removing existing containers...'
# docker stop trader
docker rm -f $(docker ps -a -q);

if [ ! -z "$REMOVE_IMAGES" ]; then
    echo '... and images.'
    docker rmi $(docker images -a -q);
fi

/root/start.sh

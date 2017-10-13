#!/usr/bin/bash

while true; do
    sleep 5
    if [[ $(docker ps -a | grep Exited) ]]; then
        echo `date` Restarting containers...
        KEEP_IMAGES=1 ./restart.sh
    fi
done

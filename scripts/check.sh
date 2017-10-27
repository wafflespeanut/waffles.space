#!/usr/bin/bash

if [[ $(docker ps -a | tail --lines=+2 | wc -l) -ne $(docker ps -a | tail --lines=+2 | grep 'Up' | wc -l) ]]; then
    echo `date` Restarting containers...
    KEEP_IMAGES=1 /home/core/restart.sh
    echo `date` >> /home/core/watcher.log
fi

#/usr/bin/bash

cd ~/source

reclone() {
    if [ -d $1 ]; then
        rm -rf $1
    fi

    git clone $2 $1
    rm -rf $1/.git
}

reclone AISH git://github.com/wafflespeanut/AISH
reclone flight-2016 git://github.com/wafflespeanut/flight-2016
reclone _site git://github.com/wafflespeanut/waffles.space

cp -r _site/source/* .
rm -rf _site

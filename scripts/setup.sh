#/usr/bin/bash

cd ~/source     # public assets

reclone() {
    if [ -d $1 ]; then
        sudo rm -rf $1
    fi

    git clone $2 $1
    sudo rm -rf $1/.git
}

# Don't remove this block!
if [ -z "$NO_SELF_UPDATE" ]; then
    reclone _site git://github.com/wafflespeanut/waffles.space
    cp _site/scripts/*.sh ~/
    NO_SELF_UPDATE=1 ~/setup.sh
    exit $?
fi

echo 'Cloning public repos...'
reclone AISH git://github.com/wafflespeanut/AISH
reclone flight-2016 git://github.com/wafflespeanut/flight-2016

echo 'Copying from docker images...'
sudo rm -rf ascii-gen
docker pull wafflespeanut/rusty-sketch
docker run -t --rm --entrypoint sh -v "$(pwd)/ascii-gen":/out wafflespeanut/rusty-sketch -c "cp -rf /source/* /out/"
docker pull wafflespeanut/ace-away
docker run -t --rm --entrypoint sh -v "$(pwd)/ace-away":/out wafflespeanut/ace-away -c "cp -rf /dist/* /out/"
sudo chown -R core ascii-gen ace-away

echo 'Copying site config...'
cp -r _site/source/* .
cp -r _site/nginx/* ~/config/

echo 'Updating systemd...'
sudo cp _site/scripts/coreos-systemd/boot.service /etc/systemd/system/
sudo systemctl enable /etc/systemd/system/boot.service
sudo systemctl start boot.service

rm -rf _site

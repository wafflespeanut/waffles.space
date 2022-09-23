#/bin/bash

cd /root/source     # public assets

reclone() {
    if [ -d $1 ]; then
        sudo rm -rf $1
    fi

    git clone $2 $1
    sudo rm -rf $1/.git
}

# Don't remove this block!
if [ -z "$NO_SELF_UPDATE" ]; then
    reclone _site https://github.com/wafflespeanut/waffles.space
    cp _site/scripts/*.sh /root/
    NO_SELF_UPDATE=1 /root/setup.sh
    exit $?
fi

echo 'Cloning public repos...'
reclone AISH https://github.com/wafflespeanut/AISH
reclone flight-2016 https://github.com/wafflespeanut/flight-2016

echo 'Copying from docker images...'
sudo rm -rf ascii-gen
docker pull wafflespeanut/rusty-sketch
docker run -t --rm --entrypoint sh -v "/root/ascii-gen":/out wafflespeanut/rusty-sketch -c "cp -rf /source/* /out/"
docker pull wafflespeanut/ace-away
docker run -t --rm --entrypoint sh -v "/root/ace-away":/out wafflespeanut/ace-away -c "cp -rf /dist/* /out/"
docker pull wafflespeanut/oi-vol-perf
mkdir -p private/oi-vol-perf
docker run -t --rm --entrypoint sh -v "/root/private/oi-vol-perf":/out wafflespeanut/oi-vol-perf -c "cp -rf /static/* /out/"
docker pull wafflespeanut/cryptofolio
mkdir -p private/cryptofolio
docker run -t --rm --entrypoint sh -v "/root/private/cryptofolio":/out wafflespeanut/cryptofolio -c "cp -rf /static/* /out/"

echo 'Copying site config...'
cp -r _site/source/* .
cp -r _site/nginx/* /root/config/

echo 'Updating systemd...'
sudo cp _site/scripts/systemd/*.service /etc/systemd/system/
sudo systemctl enable /etc/systemd/system/boot.service
sudo systemctl start boot.service

rm -rf _site

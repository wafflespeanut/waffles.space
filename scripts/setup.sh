#/usr/bin/bash

cd ~/source

reclone() {
    if [ -d $1 ]; then
        sudo rm -rf $1
    fi

    git clone $2 $1
    rm -rf $1/.git
}

echo 'Cloning public repos...'
reclone AISH git://github.com/wafflespeanut/AISH
reclone flight-2016 git://github.com/wafflespeanut/flight-2016
reclone _site git://github.com/wafflespeanut/waffles.space
reclone _ascii git://github.com/wafflespeanut/ascii-art-generator

echo 'Building ASCII art generator...'
rm -rf ascii-gen
cd _ascii
sudo chmod 777 .
docker run --rm -it -v "$(pwd)":/home/rust/src wafflespeanut/rust-wasm-builder:nightly wasm-pack build
docker run --rm -it -v "$(pwd)":/home/node/app node sh -c "cd /home/node/app/pkg && npm link && cd .. && npm link rusty-sketch && npm install && npm run build"
sudo chown -R core .
cp -r .build ../ascii-gen
cd .. && rm -rf _ascii

echo 'Copying site config...'
cp -r _site/source/* .
cp -r _site/nginx/* ~/config/

echo 'Updating systemd...'
sudo cp _site/scripts/coreos-systemd/boot.service /etc/systemd/system/
sudo systemctl enable /etc/systemd/system/boot.service
sudo systemctl start boot.service

cp _site/scripts/*.sh ~/
rm -rf _site

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ALIAS_ADDR="10.0.0.10"

if [ -z "$HostIP" ]; then
    echo "Please set \$HostIP"
    exit 1
fi

args=( "-o" "UserKnownHostsFile=/dev/null" "-o" "StrictHostKeyChecking=no" )

execute() {
    ssh "${args[@]}" core@${HostIP} "$1" 2> /dev/null
}

echo 'Removing existing images, containers and content...'
execute 'docker rmi $(docker images -a -q); \
         docker rm -f $(docker ps -a -q); \
         rm -rf ~/*'

# `localhost` and `0.0.0.0` conflicts with docker containers. It's better
# to alias the IP address (that's what Kubernetes does).
execute "sudo ifconfig eth0:1 ${ALIAS_ADDR} \
         netmask 255.255.255.0 up"

echo 'Launching static file server...'
scp -r "${args[@]}" $DIR/source/ core@${HostIP}:~/
execute "docker run --name static \
    -v ~/source:/source \
    -p ${ALIAS_ADDR}:8000:8000 \
    -e SOURCE=/source -d \
    wafflespeanut/static-server"

echo 'Launching ASCII art generator...'
execute "docker run --name ascii-gen \
    -p ${ALIAS_ADDR}:5000:5000 -d \
    wafflespeanut/ascii-gen"

echo 'Deploying Nginx proxy...'
execute "mkdir -p ~/nginx"
scp "${args[@]}" $DIR/nginx/nginx.conf core@${HostIP}:~/nginx/
scp "${args[@]}" $DIR/nginx/default.conf core@${HostIP}:~/nginx/

execute "docker run --name nginx \
    -v ~/nginx/nginx.conf:/etc/nginx/nginx.conf:ro \
    -v ~/nginx/default.conf:/etc/nginx/conf.d/default.conf:ro \
    -p 80:80 -d \
    nginx:1.13-alpine"

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ALIAS_ADDR="10.0.0.10"

if [ -z "$HostIP" ]; then
    echo "Please set \$HostIP"
    exit 1
fi

args=( "-o" "UserKnownHostsFile=/dev/null" "-o" "StrictHostKeyChecking=no" )

# `localhost` and `0.0.0.0` conflicts with docker containers. It's better
# to alias the IP address (that's what Kubernetes does).
ssh "${args[@]}" core@${HostIP} \
    "sudo ifconfig eth0:1 ${ALIAS_ADDR} \
     netmask 255.255.255.0 up"

echo 'Launching static file server...'
ssh "${args[@]}" core@${HostIP} "mkdir -p source"
ssh "${args[@]}" core@${HostIP} \
    "docker run --name static \
        -v ~/source:/source
        -p ${ALIAS_ADDR}:8000:8000 -d \
        wafflespeanut/static-server"

echo 'Launching ASCII art generator...'
ssh "${args[@]}" core@${HostIP} \
    "docker run --name ascii-gen \
        -p ${ALIAS_ADDR}:5000:5000 -d \
        wafflespeanut/ascii-gen"

echo 'Deploying Nginx proxy...'
ssh "${args[@]}" core@${HostIP} "mkdir -p ~/nginx"
scp "${args[@]}" $DIR/nginx/nginx.conf core@${HostIP}:~/nginx/
scp "${args[@]}" $DIR/nginx/default.conf core@${HostIP}:~/nginx/

ssh "${args[@]}" core@${HostIP} \
    "docker run --name nginx \
        -v ~/nginx/nginx.conf:/etc/nginx/nginx.conf:ro \
        -v ~/nginx/default.conf:/etc/nginx/conf.d/default.conf:ro \
        -p 80:80 -d \
        nginx:1.13-alpine"

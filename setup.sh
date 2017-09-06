DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ASCII_GEN_ADDR="10.0.0.10"

if [ -z "$HostIP" ]; then
    echo "Please set \$HostIP"
    exit 1
fi

args=( "-o" "UserKnownHostsFile=/dev/null" "-o" "StrictHostKeyChecking=no" )

echo 'Launching ASCII art generator...'
# `localhost` and `0.0.0.0` conflicts with docker containers. It's better
# to alias the IP address (that's what Kubernetes does).
ssh "${args[@]}" core@${HostIP} \
    "sudo ifconfig eth0:1 ${ASCII_GEN_ADDR} \
     netmask 255.255.255.0 up"
ssh "${args[@]}" core@${HostIP} \
    "docker run --name ascii-gen \
        -p ${ASCII_GEN_ADDR}:80:5000 -d \
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

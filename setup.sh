DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

if [ -z "$HostIP" ]; then
    echo "Please set \$HostIP"
    exit 1
fi

args=( "-o" "UserKnownHostsFile=/dev/null" "-o" "StrictHostKeyChecking=no" )

ssh "${args[@]}" core@${HostIP} "mkdir -p ~/nginx"
scp "${args[@]}" $DIR/nginx/nginx.conf core@${HostIP}:~/nginx/
scp "${args[@]}" $DIR/nginx/default.conf core@${HostIP}:~/nginx/

ssh "${args[@]}" core@${HostIP} \
    "docker run --name nginx \
        -v ~/nginx/nginx.conf:/etc/nginx/nginx.conf:ro \
        -v ~/nginx/default.conf:/etc/nginx/conf.d/default.conf:ro \
        -p 80:80 -d \
        nginx:1.13-alpine"

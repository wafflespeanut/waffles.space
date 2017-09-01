### Deployment scripts for [my website](https://waffles.space).

 - All my apps are in docker containers, and are managed by Kubernetes. A [script](https://github.com/wafflespeanut/coreos-kube-deploy) helps with the initial setup.
 - Then, I `kubectl create` all my apps.
 - Right now, I use `nginx/setup.sh` to launch Nginx proxy in a docker container in the same machine which runs the Kubernetes API server (I plan to move this into the kube cluster once I have a load balancer in place).

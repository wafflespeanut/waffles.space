## [waffles.space](https://waffles.space)

This repo contains the deployment scripts for my website. All my apps are in docker containers, exposed outside through an Nginx proxy (which is also a docker container). I run `HostIp=${IPv4} setup.sh` to deploy them.

### Future on Kubernetes

I'm planning to move into Kubernetes at some point. I already [have a script](https://github.com/wafflespeanut/waffles.space/tree/master/kubernetes-digitalocean) for deploying Kubernetes in CoreOS machines in DigitalOcean. But, when it comes to single-node cluster of master and worker, it seems to cause *indeterministic* issues every now and then. So, until I set up multi-node clusters, I'll be sticking with docker containers.

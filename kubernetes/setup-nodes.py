import getpass, json, os, requests, subprocess, time

from argparse import ArgumentParser

CONFIG_FILE = 'config.json'
DEFAULT_SERVICE_IP = '10.3.0.1'

DOCKER_ETCD_COMMAND = \
'''docker run -d -v /usr/share/ca-certificates/:/etc/ssl/certs -p 4001:4001 -p 2380:2380 -p 2379:2379 \
 --name etcd quay.io/coreos/etcd:v2.3.8 \
 -name etcd0 \
 -advertise-client-urls http://{HostIP}:2379,http://{HostIP}:4001 \
 -listen-client-urls http://0.0.0.0:2379,http://0.0.0.0:4001 \
 -initial-advertise-peer-urls http://{HostIP}:2380 \
 -listen-peer-urls http://0.0.0.0:2380 \
 -initial-cluster-token etcd0 \
 -initial-cluster etcd0=http://{HostIP}:2380 \
 -initial-cluster-state new'''

# FIXME: Support multiple master nodes (HA configuration)
API_CONF = \
'''[req]
req_extensions = v3_req
distinguished_name = req_distinguished_name
[req_distinguished_name]
[ v3_req ]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
subjectAltName = @alt_names
[alt_names]
DNS.1 = kubernetes
DNS.2 = kubernetes.default
DNS.3 = kubernetes.default.svc
DNS.4 = kubernetes.default.svc.cluster.local
IP.1 = {ServiceIP}
IP.2 = {HostIP}'''

WORKER_CONF = \
'''[req]
req_extensions = v3_req
distinguished_name = req_distinguished_name
[req_distinguished_name]
[ v3_req ]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
subjectAltName = @alt_names
[alt_names]
IP.1 = {WorkerIP}'''

def run_command(cmd):
    # print '\033[95m' + ' '.join(cmd) + '\033[0m'
    with open(os.devnull, 'w') as devnull:
        return subprocess.check_output(cmd, stderr=devnull)


class KubernetesCertificateProvider(object):
    def __init__(self, path):
        if not os.path.isdir(path):
            os.makedirs(path)
        self.path = path

    def generate_paths_for_cert(self, prefix):
        return (os.path.join(self.path, '%s-key.pem' % prefix),
                os.path.join(self.path, '%s.csr' % prefix),
                os.path.join(self.path, '%s.pem' % prefix),
                os.path.join(self.path, '%s.cnf' % prefix))

    def generate_private_key(self, path):
        print '\nGenerating private key...'
        run_command(['openssl', 'genrsa', '-out', path, '2048'])

    def create_root_ca(self):
        self.ca_key, _, self.ca_cert, _ = self.generate_paths_for_cert('ca')
        self.generate_private_key(self.ca_key)
        print 'Generating CA root certificate...'
        run_command(['openssl', 'req', '-x509', '-new', '-nodes',
                     '-key', self.ca_key, '-days', '10000', '-out', self.ca_cert,
                     '-subj', '/CN=kube-ca'])

    def create_signed_key_pair(self, prefix, config=None, cert_expiry_days=365):
        priv_key_path, cert_path, signed_path, conf_path = \
            self.generate_paths_for_cert(prefix)
        if config is not None:
            with open(conf_path, 'w') as fd:
                fd.write(config)

        self.generate_private_key(priv_key_path)
        print 'Generating certificate for %s...' % prefix
        cmd = ['openssl', 'req', '-new', '-key', priv_key_path,
               '-out', cert_path, '-subj', '/CN=%s' % prefix]
        if config is not None:
            cmd.extend(['-config', conf_path])
        run_command(cmd)

        print 'Signing the certificate with CA private key...'
        cmd = ['openssl', 'x509', '-req', '-in', cert_path, '-CA', self.ca_cert,
               '-CAkey', self.ca_key, '-CAcreateserial', '-out', signed_path,
               '-days', str(cert_expiry_days)]
        if config is not None:
            cmd.extend(['-extensions', 'v3_req', '-extfile', conf_path])

        run_command(cmd)
        if config is not None:
            os.remove(conf_path)

        return priv_key_path, cert_path, signed_path


class DigitalOceanKubeRunner(object):
    root_url = 'https://api.digitalocean.com/v2'
    ssh_url = root_url + '/account/keys'
    region_url = root_url + '/regions'
    droplet_url = root_url + '/droplets'
    headers = {
        'Content-Type': 'application/json'
    }

    def __init__(self, config):
        self.config = config
        self.headers['Authorization'] = 'Bearer %s' % config['api-token']
        public_key_path = os.path.expanduser(config['ssh-key-path'])
        with open(public_key_path, 'r') as fd:
            self.pkey = fd.read().strip()

        certs_path = os.path.expanduser(self.config['certs-path'])
        self.cert_provider = KubernetesCertificateProvider(certs_path)
        self.cert_provider.create_root_ca()

    def node_creation_request(self, name, region, size, ssh_key_id):
        return {
            'name': name,
            'region': region,
            'size': size,
            'image': 'coreos-stable',
            'ssh_keys': [ ssh_key_id ],
            'backups': False,
            'ipv6': True,
            'user_data': None,
            'private_networking': True,
            'volumes': [],
            'tags': [],
        }

    def _request(self, method, url, data=None):
        if data is not None:
            data = json.dumps(data)
        req_method = getattr(requests, method.lower())
        print '%s: %s' % (method, url)
        resp = req_method(url, data=data, headers=self.headers)
        data, code = resp.text, resp.status_code

        if code < 200 or code >= 300:
            print 'Got %s response: %s' % (code, data)
            raise Exception('Boo!')
        return json.loads(data)

    def create_or_use_public_key(self):
        data = self._request('GET', self.ssh_url)
        keys = filter(lambda k: self.pkey == k['public_key'], data['ssh_keys'])

        if keys:
            key = keys[0]
        else:
            print 'Cannot find key in the cloud. Creating new key...'
            payload = {
                'name': "%s's key" % getpass.getuser(),
                'key': self.pkey
            }
            data = self._request('POST', self.ssh_url, data=payload)
            key = data['ssh_key']

        key = keys[0]
        print 'Using key: %s (fingerprint: %s)' % (key['name'], key['fingerprint'])
        return key['id']

    def get_regions(self):
        data = self._request('GET', self.region_url)
        self.regions = filter(lambda r: r['available'], data['regions'])

    def get_droplets(self):
        data = self._request('GET', self.droplet_url)
        self.droplets = data['droplets']

    def get_public_ip_for_droplet(self, node_data):
        ip = filter(lambda ip: ip['type'] == 'public', node_data['networks']['v4'])
        return ip[0]['ip_address']

    def get_or_create_node(self, ssh_key_id, node, node_id=0):
        size = self.config['size']
        regions = filter(lambda r: size in r['sizes'], self.regions)
        if not regions:
            exit('No regions available for size %s' % size)

        region = regions[0]['slug']
        name = 'coreos-%s-%s-%s' % (node, node_id, region)
        droplet = filter(lambda d: d['name'] == name, self.droplets)
        if droplet:
            print 'Re-using existing droplet...'
            return droplet[0]

        payload = self.node_creation_request(name, region, size, ssh_key_id)
        print 'Creating droplet %s with size %s...' % (name, size)
        data = self._request('POST', self.droplet_url, payload)
        droplet_id = data['droplet']['id']
        status = data['droplet']['status']

        print 'Waiting for droplet...'
        url = self.droplet_url + '/' + str(droplet_id)
        while status != 'active':
            time.sleep(5)
            data = self._request('GET', url)
            status = data['droplet']['status']

        self.droplets.append(data['droplet'])
        return data['droplet']

    def run_command_in_node(self, node_ip, cmd):
        return run_command(['ssh', '-o',
                            'StrictHostKeyChecking no',
                            'core@%s' % node_ip, cmd])

    # FIXME:
    # - We need a 3-node etcd cluster.
    # - etcd communication should be secure.
    def run_etcd_cluster_in_node(self, host_ip):
        try:
            print '\nChecking etcd cluster in %s...' % host_ip
            resp = requests.get("http://%s:2379/version" % host_ip)
            if resp.status_code >= 200 and resp.status_code < 300:
                print 'Note: etcd cluster exists in node.'
            else:
                raise Exception
        except Exception:
            print 'Launching new etcd cluster...'
            print self.run_command_in_node(host_ip, DOCKER_ETCD_COMMAND.format(HostIP=pub_ip))

    def create_api_server_key_pair(self, host_ip):
        config = API_CONF.format(HostIP=host_ip, ServiceIP=DEFAULT_SERVICE_IP)
        return self.cert_provider.create_signed_key_pair('kube-apiserver', config)

    def create_worker_key_pair(self, worker_fqdn, worker_ip):
        config = WORKER_CONF.format(WorkerIP=worker_ip)
        return self.cert_provider.create_signed_key_pair(worker_fqdn, config)

    def create_admin_key_pair(self):
        return self.cert_provider.create_signed_key_pair('kube-admin')

    def deploy_nodes(self):
        ssh_key_id = self.create_or_use_public_key()
        self.get_regions()
        self.get_droplets()

        master = self.get_or_create_node(ssh_key_id, 'master')
        master_ip = self.get_public_ip_for_droplet(master)
        self.run_etcd_cluster_in_node(master_ip)
        api_key, api_cert, api_signed = self.create_api_server_key_pair(master_ip)

        # FIXME: Support multiple workers
        worker = self.get_or_create_node(ssh_key_id, 'worker')
        worker_ip = self.get_public_ip_for_droplet(worker)
        worker_key, worker_cert, worker_signed = self.create_worker_key_pair(worker['name'], worker_ip)
        admin_key, admin_cert, admin_signed = self.create_admin_key_pair()

if __name__ == '__main__':
    parser = ArgumentParser(description='Deploy kubernetes in Digital Ocean.')
    parser.add_argument('--config', '-c', help='Configuration file path (defaults to %r)' % CONFIG_FILE)
    parser.set_defaults(config=CONFIG_FILE)
    args = parser.parse_args()

    with open(args.config, 'r') as fd:
        config = json.load(fd)

    runner = DigitalOceanKubeRunner(config)
    runner.deploy_nodes()

import getpass, json, os, requests, subprocess, time, urllib2

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

SHELL_REPLACEMENTS = {}
MASTER_SCRIPT_URL = 'https://github.com/coreos/coreos-kubernetes/raw/master/multi-node/generic/controller-install.sh'
WORKER_SCRIPT_URL = 'https://github.com/coreos/coreos-kubernetes/raw/master/multi-node/generic/worker-install.sh'

def run_command(cmd, async=False):
    # print '\033[95m' + ' '.join(cmd) + '\033[0m'
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if async:
        return lambda: proc.communicate()[0]
    else:
        out, _err = proc.communicate()
        return out


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
               '-out', cert_path, '-subj', '/CN=kube-%s' % prefix]
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

        return priv_key_path, signed_path


class DigitalOceanKubeRunner(object):
    root_url = 'https://api.digitalocean.com/v2'
    ssh_url = root_url + '/account/keys'
    region_url = root_url + '/regions'
    droplet_url = root_url + '/droplets'
    machine_cert_path = '/etc/kubernetes/ssl/'
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

    def get_or_create_node(self, ssh_key_id, node, node_id=0, override=False):
        size = self.config['size']
        regions = filter(lambda r: size in r['sizes'], self.regions)
        if not regions:
            exit('No regions available for size %s' % size)

        region = regions[0]['slug']
        name = 'coreos-%s-%s-%s' % (node, node_id, region)
        print '\nLooking for droplet %s...' % name
        droplet = filter(lambda d: d['name'] == name, self.droplets)
        status = None

        if droplet:
            print 'Re-using existing droplet...'
            if not override:
                return droplet[0]
            # Rebuild the existing droplet
            droplet_id = droplet[0]['id']
            url = self.droplet_url + '/%s/actions' % droplet_id
            self._request('POST', url, {
                'type': 'rebuild',
                'image': 'coreos-stable'
            })
        else:
            payload = self.node_creation_request(name, region, size, ssh_key_id)
            print 'Creating droplet %s with size %s...' % (name, size)
            data = self._request('POST', self.droplet_url, payload)
            droplet_id = data['droplet']['id']

        print 'Waiting for droplet...'
        url = self.droplet_url + '/' + str(droplet_id)
        while status != 'active':
            time.sleep(5)
            data = self._request('GET', url)
            status = data['droplet']['status']

        if not droplet:
            self.droplets.append(data['droplet'])
        return data['droplet']

    def run_command_in_node(self, node_ip, cmd, async=False):
        return run_command(['ssh', '-o',
                            'StrictHostKeyChecking=no',
                            'core@%s' % node_ip, cmd], async=async)

    def send_files_to_node_home(self, node_ip, *files):
        cmd = ['scp', '-o', 'StrictHostKeyChecking=no']
        cmd.extend(files)
        print 'Sending %s file(s) to %s...' % (len(files), node_ip)
        cmd.append('core@%s:~' % node_ip)
        return run_command(cmd)

    # FIXME:
    # - We need a 3-node etcd cluster.
    # - etcd communication should be secure.
    def run_etcd_cluster_in_node(self, host_ip):
        try:
            print '\nChecking etcd cluster in %s...' % host_ip
            resp = requests.get("http://%s:2379/version" % host_ip, timeout=5)
            if resp.status_code >= 200 and resp.status_code < 300:
                print 'Note: etcd cluster exists in node.'
            else:
                raise Exception
        except Exception:
            print 'Launching new etcd cluster...'
            print self.run_command_in_node(host_ip, DOCKER_ETCD_COMMAND.format(HostIP=host_ip))

    def create_api_server_key_pair(self, host_ip):
        config = API_CONF.format(HostIP=host_ip, ServiceIP=DEFAULT_SERVICE_IP)
        return self.cert_provider.create_signed_key_pair('apiserver', config)

    def create_worker_key_pair(self, worker_fqdn, worker_ip):
        config = WORKER_CONF.format(WorkerIP=worker_ip)
        return self.cert_provider.create_signed_key_pair(worker_fqdn, config)

    def create_admin_key_pair(self):
        return self.cert_provider.create_signed_key_pair('admin')

    def configure_kubectl(self, master_ip):
        admin_key, admin_cert = self.create_admin_key_pair()
        run_command(['kubectl', 'config', 'set-cluster', 'default-cluster',
                     '--server=https://%s' % master_ip,
                     '--certificate-authority=%s' % self.cert_provider.ca_cert])
        run_command(['kubectl', 'config', 'set-credentials', 'default-admin',
                     '--certificate-authority=%s' % self.cert_provider.ca_cert,
                     '--client-key=%s' % admin_key, '--client-certificate=%s' % admin_cert])
        run_command(['kubectl', 'config', 'set-context', 'default-system',
                     '--cluster=default-cluster', '--user=default-admin'])
        run_command(['kubectl', 'config', 'use-context', 'default-system'])
        print 'kubectl is now configured to use the cluster.'

    def set_certs_in_node(self, node_ip):
        self.run_command_in_node(node_ip, 'sudo mv /home/core/*.pem %s' % self.machine_cert_path)
        self.run_command_in_node(node_ip, 'sudo chmod 600 %s' % os.path.join(self.machine_cert_path, '*-key.pem'))
        self.run_command_in_node(node_ip, 'sudo chown root:root %s' % os.path.join(self.machine_cert_path, '*-key.pem'))

    def execute_script_in_node(self, host_ip, url, async=False):
        lines = urllib2.urlopen(url).readlines()
        for i, line in enumerate(lines):
            line = line.strip()
            if line.startswith('export ') and line.endswith('='):
                var = line.split(' ')[1][:-1]
                lines[i] = 'export %s=%s\n' % (var, SHELL_REPLACEMENTS[var])

        script_path = '/tmp/init_script.sh'
        with open(script_path, 'w') as fd:
            fd.writelines(lines)

        self.send_files_to_node_home(host_ip, script_path)
        script_path = '/home/core/init_script.sh'
        self.run_command_in_node(host_ip, 'chmod +x %s' % script_path)
        print 'Launching %s in %s...' % (script_path, host_ip)
        return self.run_command_in_node(host_ip, 'sudo ' + script_path, async=async)

    def deploy_nodes(self, override=False):
        ssh_key_id = self.create_or_use_public_key()
        self.get_regions()
        self.get_droplets()

        # FIXME: Support multiple master nodes (HA configuration)
        master = self.get_or_create_node(ssh_key_id, 'master', override=override)
        master_ip = self.get_public_ip_for_droplet(master)
        self.run_etcd_cluster_in_node(master_ip)
        SHELL_REPLACEMENTS['ETCD_ENDPOINTS'] = 'http://%s:2379' % master_ip
        SHELL_REPLACEMENTS['CONTROLLER_ENDPOINT'] = 'https://%s:8080' % master_ip

        api_key, api_cert = self.create_api_server_key_pair(master_ip)
        self.run_command_in_node(master_ip, 'sudo mkdir -p %s' % self.machine_cert_path)
        self.send_files_to_node_home(master_ip, self.cert_provider.ca_cert, api_key, api_cert)
        self.set_certs_in_node(master_ip)
        master_wait = self.execute_script_in_node(master_ip, MASTER_SCRIPT_URL, async=True)

        # FIXME: Support multiple workers
        worker = self.get_or_create_node(ssh_key_id, 'worker', override=override)
        worker_ip = self.get_public_ip_for_droplet(worker)
        worker_key, worker_cert = self.create_worker_key_pair(worker['name'], worker_ip)
        self.run_command_in_node(worker_ip, 'sudo mkdir -p %s' % self.machine_cert_path)
        self.send_files_to_node_home(worker_ip, self.cert_provider.ca_cert, worker_key, worker_cert)
        self.set_certs_in_node(worker_ip)
        self.execute_script_in_node(worker_ip, WORKER_SCRIPT_URL)

        print 'Waiting for the scripts to finish...'
        master_wait()
        self.configure_kubectl(master_ip)


if __name__ == '__main__':
    parser = ArgumentParser(description='Deploy kubernetes in Digital Ocean.')
    parser.add_argument('--config', '-c', help='Configuration file path (defaults to %r)' % CONFIG_FILE)
    parser.set_defaults(config=CONFIG_FILE)
    args = parser.parse_args()

    with open(args.config, 'r') as fd:
        config = json.load(fd)

    runner = DigitalOceanKubeRunner(config)
    runner.deploy_nodes(override=True)

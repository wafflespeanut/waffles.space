import getpass, json, os, requests, time

from argparse import ArgumentParser

CONFIG_FILE = 'config.json'

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
            return droplet[0]['id']

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
        return droplet_id

    def start(self):
        ssh_key_id = self.create_or_use_public_key()
        self.get_regions()
        self.get_droplets()
        master_id = self.get_or_create_node(ssh_key_id, 'master')
        worker_id = self.get_or_create_node(ssh_key_id, 'worker')

if __name__ == '__main__':
    parser = ArgumentParser(description='Deploy kubernetes in Digital Ocean.')
    parser.add_argument('--config', '-c', help='Configuration file path (defaults to %r)' % CONFIG_FILE)
    parser.set_defaults(config=CONFIG_FILE)
    args = parser.parse_args()

    with open(args.config, 'r') as fd:
        config = json.load(fd)

    runner = DigitalOceanKubeRunner(config)
    runner.start()

import getpass, json, os, requests

from argparse import ArgumentParser

CONFIG_FILE = 'config.json'

NODE_CREATION_REQUEST = {
    'name': None,
    'region': None,
    'size': None,
    'image': 'coreos-stable',
    'ssh_keys': [],
    'backups': False,
    'ipv6': True,
    'user_data': None,
    'private_networking': True,
    'volumes': [],
    'tags': [],
}

class DigitalOceanKubeRunner(object):
    root_url = 'https://api.digitalocean.com/v2'
    ssh_url = root_url + '/account/keys'
    headers = {
        'Content-Type': 'application/json'
    }

    def __init__(self, config):
        self.config = config
        self.headers['Authorization'] = 'Bearer %s' % config['api-token']
        public_key_path = os.path.expanduser(config['ssh-key-path'])
        with open(public_key_path, 'r') as fd:
            self.pkey = fd.read().strip()

    def _request(self, method, url, data=None):
        if data is not None:
            data = json.dumps(data)
        req_method = getattr(requests, method.lower())
        print '%s: %s' % (method, url)
        resp = req_method(url, data=data, headers=self.headers)
        data, code = resp.text, resp.status_code

        if code < 200 or code >= 300:
            print 'Got a %s response: %s' % (code, data)
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

    def start(self):
        ssh_key_id = self.create_or_use_public_key()


if __name__ == '__main__':
    parser = ArgumentParser(description='Deploy kubernetes in Digital Ocean.')
    parser.add_argument('--config', '-c', help='Configuration file path (defaults to %r)' % CONFIG_FILE)
    parser.set_defaults(config=CONFIG_FILE)
    args = parser.parse_args()

    with open(args.config, 'r') as fd:
        config = json.load(fd)

    runner = DigitalOceanKubeRunner(config)
    runner.start()

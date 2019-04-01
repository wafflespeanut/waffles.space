from flask import Flask, request

import imp
import os

if __name__ == '__main__':
    app = Flask('Callbacks server')
    handlers = {}
    handler_dir = os.path.dirname(os.path.realpath(__file__))
    secret = os.getenv('SECRET')
    if not secret:
        exit('SECRET not set in env!')

    for mod_path in os.listdir(handler_dir):
        if not mod_path.endswith('.py'):
            continue

        mod_name = os.path.basename(mod_path).split('.')[0]
        mod_path = os.path.join(handler_dir, mod_path)
        print 'Loading', mod_path
        module = imp.load_source(mod_name, mod_path)

        try:
            module.init()
            handlers[mod_name] = module.handle
        except AttributeError:
            continue

    @app.route('/', methods=['GET'])
    def callback():
        if request.args.get('secret', '') != secret:
            return app.response_class(response="Oops!", status=403)

        name = request.args.get('handler', '')
        handler = handlers.get(name)
        if handler is None:
            print 'No matching handler for', name
            return app.response_class(response="Sorry!", status=404)

        handler(request.args)
        return app.response_class(response="Okay!", status=202)

    port = int(os.environ.get('PORT', 7777))
    app.run(host='0.0.0.0', port=port)

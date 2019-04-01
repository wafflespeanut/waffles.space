import logging
import os
from datetime import datetime
from skpy import Skype

CLIENT = None
init_time = datetime.now()

TOKEN_FILE = '.token-skype'
with open(TOKEN_FILE, 'w'):     # workaround
    pass

def init():
    u, p = os.getenv('SKYPE_USER'), os.getenv('SKYPE_PASS')
    if not u or not p:
        logging.info('Missing credentials. Skipping client init')
        return

    try:
        isEmpty = False
        with open(TOKEN_FILE, 'r') as fd:
            buf = fd.read()
            isEmpty = len(buf) == 0

        if isEmpty:
            CLIENT = Skype(u, p, TOKEN_FILE)
        else:
            CLIENT = Skype(tokenFile=TOKEN_FILE)
    except Exception as err:
        if not isEmpty:     # invalid token
            with open(TOKEN_FILE, 'w') as fd:
                fd.truncate(0)

        logging.error('Error creating client: %r', err)

def handle(kwargs):
    global CLIENT
    now = datetime.now()
    if (now - init_time).seconds > 1200:
        CLIENT = None

    if CLIENT is None:
        logging.info('Trying to initialize client...')
        init()

    if CLIENT is None:
        return

    conv, msg = kwargs.get('conv'), kwargs.get('msg')
    if not conv or not msg:
        logging.info('Invalid conversation or message.')
        return

    try:
        conv = CLIENT.chats.chat(conv)
        conv.sendMsg(msg)
    except Exception as err:
        logging.error("Error sending message: %r", err)

import os
from datetime import datetime
from skpy import Skype

CLIENT = None
init_time = datetime.now()

def init():
    u, p = os.getenv('SKYPE_USER'), os.getenv('SKYPE_PASS')
    if not u or not p:
        print 'Missing credentials. Skipping client init'
        return
    try:
        CLIENT = Skype(u, p)
    except:
        pass

def handle(kwargs):
    now = datetime.now()
    if (now - init_time).seconds > 1200:
        print 'Re-initializing client...'
        init()

    if CLIENT is None:
        print 'Missing client. Skipping callback request.'
        return

    try:
        conv, msg = kwargs.get('conv'), kwargs.get('msg')
        if not conv or not msg:
            print 'Invalid conversation or message.'
            return

        conv = CLIENT.chats.chat(conv)
        conv.sendMsg(msg)
    except:
        print 'Error sending message!'

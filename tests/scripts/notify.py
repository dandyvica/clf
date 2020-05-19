#!/usr/bin/python3
import os
import sys
import time

# get list of variables for CLF
clf = [f"{v}={os.environ.get(v)}" for v in os.environ if v.startswith("CLF")]
all_vars = "\n".join(clf)

# notify Cinnamon
#os.system(f'notify-send -t 60000 ' + all_vars)
output = open('/tmp/notify.output', 'w').write(all_vars)

//time.sleep(10)


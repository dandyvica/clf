#!/usr/bin/python3
import os

# get list of variables for CLF
clf = [f"{v}={os.environ.get(v)}" for v in os.environ if v.startswith("P")]
all_vars = "\n".join(clf)

# notify Cinnamon
os.system(f'notify-send -t 60 "{all_vars}"')


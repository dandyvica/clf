#!/usr/bin/env python3
import os
import sys
import time

# get list of variables for CLF
clf = [f"{v}=<{os.environ.get(v)}>" for v in os.environ if v.startswith("CLF")]
pid = os.getpid()

# build file into temporary directory with the first argument as the file name
output = open(sys.argv[1], "a+")
output.write(f"{pid}-{sys.argv[1]}-{clf}\n")

sys.exit(100)
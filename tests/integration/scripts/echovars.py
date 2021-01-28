#!/usr/bin/env python3
import os
import sys
import time
import tempfile

# get list of variables for CLF
clf = [f"{v}=<{os.environ.get(v)}>" for v in os.environ if v.startswith("CLF")]

# build file into temporary directory with the first argument as the file name
tmpfile = os.path.join(sys.argv[1])
output = open(tmpfile, "a+")
output.write(f"{sys.argv[1]}-{clf}\n")

sys.exit(100)

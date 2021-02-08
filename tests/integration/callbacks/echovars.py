#!/usr/bin/env python2.7
import os
import sys
import time

# get list of variables for CLF
clf = [ "%s=%s" % (v,os.environ.get(v)) for v in os.environ if v.startswith("CLF")]
pid = os.getpid()

# build file into temporary directory with the first argument as the file name
output = open(sys.argv[1], "a+")
output.write("%s-%s-%s\n" % (pid, sys.argv[1], clf))

sys.exit(100)

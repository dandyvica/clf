#!/usr/bin/python3
import os
import sys
import time
import tempfile

# get list of variables for CLF
clf = [f"{v}=<{os.environ.get(v)}>" for v in os.environ if v.startswith("CLF_")]
all_vars = "\n".join(clf)

# just creates files with all variables

# build file into temporary directory
tmpfile = os.path.join(tempfile.gettempdir(), "echovars.txt")

output = open(tmpfile, "a+")
output.write("-"*80)
output.write(f"\nargs={sys.argv}\n")
output.write(f"\n{all_vars}\n")

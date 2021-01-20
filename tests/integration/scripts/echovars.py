#!/usr/bin/env python3
import os
import sys
import time
import tempfile

# get list of variables for CLF
clf = [f"{v}=<{os.environ.get(v)}>" for v in os.environ if v.startswith("CLF")]
#all_vars = "\n".join(clf)

# just creates files with all variables

# build file into temporary directory with the first argument as the file name
tmpfile = os.path.join(tempfile.gettempdir(), sys.argv[1])

output = open(tmpfile, "a+")
#output.write("-"*80 + "\n")
# output.write(f"\nargs={sys.argv}\n")
# output.write(f"\n{all_vars}\n")
output.write(f"{sys.argv[1]}-{clf}\n")

sys.exit(100)

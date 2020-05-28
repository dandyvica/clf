#!/usr/bin/python3
import os
import sys
import time

# get list of variables for CLF
clf = [f"{v}=<{os.environ.get(v)}>" for v in os.environ if v.startswith("CLF")]
all_vars = "\n".join(clf)

# just creates files with all variables
output = open("/tmp/echovars.txt", "a+")
output.write("-"*80)
output.write(f"\nargs={sys.argv}\n")
output.write(f"\n{all_vars}\n")

sys.exit(100)

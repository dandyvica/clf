#!/usr/bin/python3
# used to test the script call from clf
import os
import sys

# get environment variables
vars = { v:os.environ.get(v) for v in os.environ if v.startswith("CLF_") }

try:
    assert sys.argv[1:] == ['one', 'two', 'three']

    assert vars["CLF_CAPTURE1"] == "my name is"
    assert vars["CLF_CAPTURE2"] == "john"
    assert vars["CLF_CAPTURE3"] == "fitzgerald"
    assert vars["CLF_LASTNAME"] == "kennedy"

except AssertionError:
    exit(1)    
except KeyError:
    exit(1)




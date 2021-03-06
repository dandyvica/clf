#!/usr/bin/env python3
# used to test the script call from clf
import os
import sys

# get environment variables
vars = { v:os.environ.get(v) for v in os.environ if v.startswith("CLF_") }

with open('/tmp/myfile.txt', 'w') as f:
    print(sys.argv, file=f)
    print(vars, file=f)

try:
    assert sys.argv[1:] == ['one', 'two', 'three']

    assert vars["CLF_CG_1"] == "my name is"
    assert vars["CLF_CG_2"] == "john"
    assert vars["CLF_CG_3"] == "fitzgerald"
    assert vars["CLF_CG_LASTNAME"] == "kennedy"

    exit(0)

except AssertionError:
    exit(1)    
except KeyError:
    exit(1)




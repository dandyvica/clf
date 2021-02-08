#!/usr/bin/env python2.7

# simple script to kill a process by ID
import os
import sys

os.kill(int(sys.argv[1]), 9)
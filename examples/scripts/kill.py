#!/usr/bin/python3

# simple script to kill a process by ID
import os
import sys

print("killing PID " + sys.argv[1])
os.kill(int(sys.argv[1]), 9)
#!/bin/bash
set -x

clf=~/projects/clf/target/release/clf
config=~/projects/clf/tests/local/syslog.yml
log=~/projects/clf/tests/local/clf.log

# launch clf
$clf --config $config --clflog $log --delsnap --loglevel Trace
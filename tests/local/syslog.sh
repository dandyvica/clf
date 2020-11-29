#!/bin/bash
set -x

clf=~/projects/clf/target/debug/clf
config=~/projects/clf/tests/local/syslog.yml
log=~/projects/clf/tests/local/clf.log
snapshot=~/projects/clf/tests/local/clf_snapshot.json

# launch clf
$clf --config $config --logger $log  --loglevel Debug --snapshot $snapshot --delete-snapshot $@

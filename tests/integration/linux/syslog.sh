#!/bin/bash
#set -x

root=~/projects/clf

clf=$root/target/debug/clf
config=$root/tests/integration/linux/syslog.yml
log=$root/tests/integration/linux/clf.log
snapshot=$root/tests/integration/linux/clf_snapshot.json

# launch clf
$clf --config $config --log $log  --loglevel Debug --snapshot $snapshot --delete-snapshot --var FOO1:bar1 FOO2:bar2 $@

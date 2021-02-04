#!/bin/bash
#set -x
if [ -z $1 ]
then
  echo "clf.sh (script|tcp|domain)"
  exit
fi

option=$1
clf=~/projects/clf/target/release/clf
config=~/projects/clf/examples/config
log=~/projects/clf/examples/tmp/clf.log

# launch clf
$clf --config $config/$1.yml --log $log --delete-snapshot

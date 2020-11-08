#!/bin/bash
set -x
if [ -z $1 ]
then
  echo "clf.sh (script|tcp|domain)"
  exit
fi

option=$1
clf=~/projects/clf/target/debug/clf
config=~/projects/clf/tests/config
log=~/projects/clf/tests/tmp/clf.log

# launch clf
$clf --config $config/$1.yml --clflog $log --delsnap

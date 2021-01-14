#!/usr/bin/ruby
require './testcase'

# an experiment to run dedicated testcases for clf

# testcase is a YAML file as first argument
tc_file = ARGV[0]

# load YAML data
tc = TestScenario.new(tc_file)

tc.run

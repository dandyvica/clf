#!/usr/bin/ruby
require './testcase'

clf = "/home/alain/projects/clf/target/release/clf"

tc = TestCase.new("ruby", ".", clf)
tc.create_log
tc.run("-d")

config = Config.new(tc.config_file)


import sys
import os
import yaml
import json
import subprocess
import unittest
from pathlib import Path

# a generic class for testing different scenarii
class TestCase:
    # just saves the clf executable path
    def __init__(self):
        self.clf = os.path.join(Path.home(), "projects", "clf", "target", "debug", "clf")
        self.template = os.path.join(os.getcwd(), "config", "template.yml")

    # calls the clf executable with arguments
    def spawn(self, args: list):
        return subprocess.run([self.clf] + args, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    # calls the clf executable with arguments
    def exec(self, *args: tuple):
        params = [self.clf, "-c", self.config, "-p", self.snapshot, "-g", "Trace"]
        return subprocess.run(params + list(args), stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    # prepare the test case by creating the final configuration YAML file
    def prepare(self, tc_name: str, **vars: dict) -> str:
        # read template as a string
        cnf = open(self.template)
        config = cnf.read()
        cnf.close()

        # test mandatory arguments
        if "logfile" not in vars:
            raise ValueError("'logfile' is mandatory when calling prepare() method")

        if "options" not in vars:
            raise ValueError("'options' is mandatory when calling prepare() method")

        # add constant data if not present when calling this method
        if "format" not in vars:
            vars["format"] = "plain"

        if "process" not in vars:
            vars["process"] = "true"

        if "callback" not in vars:
            vars["callback"] = 'address: "127.0.0.1:8999"'

        if "args" not in vars:
            vars["args"] = "'arg1', 'arg2', 'arg3'"

        # replace data from variables
        for (k,v) in vars.items():
            config = config.replace("$"+k,v)  
            
        # build new config file name
        tc_config = os.path.join(os.getcwd(), "tmp", tc_name + ".yml")

        # write data
        tc_file = open(tc_config, "w")
        tc_file.write(config)
        tc_file.close()

        # save test case name and config name to be re-used by the exec() method
        self.config = tc_config
        self.snapshot = os.path.join(os.getcwd(), "tmp", tc_name + ".json")

    # return relevant data from snapshot
    def run_data(self) -> dict:
        snapshot_file = open(self.snapshot)
        snapshot_data = json.load(snapshot_file)
        snapshot_file.close()

        # we only have one logfile in the snapshot, no nned to get its name
        logfile = list(snapshot_data["snapshot"].keys())[0]
        return snapshot_data["snapshot"][logfile]["run_data"]["tag1"]

    # return relevant data from snapshot
    def id_data(self) -> dict:
        snapshot_file = open(self.snapshot)
        snapshot_data = json.load(snapshot_file)
        snapshot_file.close()

        # we only have one logfile in the snapshot, no nned to get its name
        logfile = list(snapshot_data["snapshot"].keys())[0]
        return snapshot_data["snapshot"][logfile]["id"]

# run tests cases for system testing clf
import os
import stat
import unittest
from pathlib import Path
from shutil import copyfile

from testcase import TestCase

class Testing(unittest.TestCase):
    # helper functions
    def assert_rundata(self, **kwargs):
        rundata = self.tc.run_data()

        for (k,v) in kwargs.items():
            self.assertEqual(rundata[k], v)

    def assert_counters(self, **kwargs):
        counters = self.tc.run_data()["counters"]

        for (k,v) in kwargs.items():
            self.assertEqual(counters[k], v)

    def newlog(self, log: str):
        if os.path.exists(log):
            os.remove(log)
        copyfile("./logfiles/access_simple.log", log)
        st = os.stat(log)
        os.chmod(log, st.st_mode | stat.S_IWRITE)

    def add_logdata(self, log: str):
        file = open(log, "a")

        for i in range(100):
            file.write('195.8.51.14 - - [13/Dec/2015:14:28:15 +0100] "GET /administrator/ HTTP/1.1" 200 4263 "-" "Mozilla/5.0 (Windows NT 6.0; rv:34.0) Gecko/20100101 Firefox/34.0" "-"')
            file.write("\n")

        file.close()

    # called once
    def setUp(self):
        self.tc = TestCase()

    # argument without really processing the logfile
    def test_help(self):
        p = self.tc.spawn(["-h"])
        self.assertEqual(p.returncode, 0)

    def test_missing_arg(self):
        p = self.tc.spawn(["-s"])
        self.assertEqual(p.returncode, 2)

    def test_syntax_check(self):
        self.tc.prepare("syntax_check", logfile="./logfiles/access_simple.log", options="fastforward")

        p = self.tc.exec("-o")
        self.assertEqual(p.returncode, 0)

    def test_show_options(self):
        self.tc.prepare("show_options", logfile="./logfiles/access_simple.log", options="fastforward")

        p = self.tc.exec("-s")
        self.assertEqual(p.returncode, 0)

    def test_yaml_error(self):
        print("test_yaml_error: todo")

    # logfile not found and related logfilemissing options
    def test_log_notfound_unkown(self):
        self.tc.prepare("log_notfound_unkown", logfile="./logfiles/access_simple.lo", options="fastforward")

        p = self.tc.exec()
        self.assertEqual(p.returncode, 3)

    def test_log_notfound_error(self):
        self.tc.prepare("log_notfound_error", logfile="./logfiles/access_simple.lo", options="logfilemissing=error")

        p = self.tc.exec()
        self.assertEqual(p.returncode, 2)

    def test_log_notfound_warning(self):
        self.tc.prepare("log_notfound_warning", logfile="./logfiles/access_simple.lo", options="logfilemissing=warning")

        p = self.tc.exec()
        self.assertEqual(p.returncode, 1)

    # regular processing of the logfile
    def test_fastforward(self):
        self.tc.prepare("fastforward", logfile="./logfiles/access_simple.log", options="fastforward,runcallback")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 0)        

        # check id data just once
        id = self.tc.id_data()
        self.assertEqual(id["extension"], "log")
        self.assertEqual(id["compression"], "uncompressed")

        # assert all run data
        self.assert_rundata(last_line=999, last_offset=197326)
        self.assert_counters(critical_count=0, warning_count=0, ok_count=0, exec_count=0)

    def test_fastforward_gz(self):
        self.tc.prepare("fastforward_gz", logfile="./logfiles/access_simple.log.gz", options="fastforward,runcallback")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 0)        

        # check id data just once
        id = self.tc.id_data()
        self.assertEqual(id["extension"], "gz")
        self.assertEqual(id["compression"], "gzip")

        # assert all run data
        self.assert_rundata(last_line=999, last_offset=197326)
        self.assert_counters(critical_count=0, warning_count=0, ok_count=0, exec_count=0)

    def test_stopat(self):
        self.tc.prepare("stopat", logfile="./logfiles/access_simple.log", options="stopat=200")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 2)     

        # assert all run data
        self.assert_rundata(last_line=200, last_offset=44292)
        self.assert_counters(critical_count=63, warning_count=2, ok_count=0, exec_count=0)

    def test_savethresholdcount(self):
        self.tc.prepare("savethresholdcount", logfile="./logfiles/access_simple.log", options="savethresholdcount")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 2)        

        # assert all run data
        self.assert_rundata(last_line=999, last_offset=197326)
        self.assert_counters(critical_count=440, warning_count=28, ok_count=0, exec_count=0)

    def test_criticalthreshold(self):
        self.tc.prepare("criticalthreshold", logfile="./logfiles/access_simple.log", options="runcallback,criticalthreshold=439,warningthreshold=100", callback="script: ./scripts/echovars.py")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 2)        

        # assert all run data
        self.assert_rundata(last_line=999, last_offset=197326)
        self.assert_counters(critical_count=440, warning_count=28, ok_count=0, exec_count=2)

    def test_warningthreshold(self):
        self.tc.prepare("warningthreshold", logfile="./logfiles/access_simple.log", options="runcallback,criticalthreshold=1000,warningthreshold=25", callback="script: ./scripts/echovars.py")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 2)        

        # assert all run data
        self.assert_rundata(last_line=999, last_offset=197326)
        self.assert_counters(critical_count=440, warning_count=28, ok_count=0, exec_count=4)

    def test_rewind(self):
        self.tc.prepare("rewind", logfile="./logfiles/access_simple.log", options="fastforward,runcallback")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 0)      

        self.tc.prepare("rewind", logfile="./logfiles/access_simple.log", options="rewind")
        p = self.tc.exec()        
        self.assertEqual(p.returncode, 2)         

        # assert all run data
        self.assert_rundata(last_line=999, last_offset=197326)
        self.assert_counters(critical_count=440, warning_count=28, ok_count=0, exec_count=0)

    def test_grow(self):
        log = "./tmp/grow.log"
        self.newlog(log)

        self.tc.prepare("rotate", logfile=log, options="fastforward")
        p = self.tc.exec("-d")

        self.add_logdata(log)
        self.tc.prepare("rotate", logfile=log, options="savethresholdcount")
        p = self.tc.exec()

        # assert all run data
        self.assert_rundata(start_line=999, start_offset=197326, last_line=1099, last_offset=213426)
        self.assert_counters(critical_count=100, warning_count=0, ok_count=0, exec_count=0)

    def test_rotate(self):
        log = "./tmp/rotate.log"
        log1 = "./tmp/rotate.log.1"
        self.newlog(log)

        print(f"inode({log}) = ", os.lstat(log)[stat.ST_INO])

        self.tc.prepare("rotate", logfile=log, options="stopat=200")

        p = self.tc.exec("-d")
        self.assertEqual(p.returncode, 2)  

        # simulates log rotation   
        os.rename(log, log1)
        print(f"inode({log1}) = ", os.lstat(log1)[stat.ST_INO])

        copyfile("./logfiles/access_simple.log", log)
        print(f"inode({log}) = ", os.lstat(log)[stat.ST_INO])

        self.tc.prepare("rotate", logfile=log, options="savethresholdcount")
        p = self.tc.exec()        
        self.assertEqual(p.returncode, 2)         

        # assert all run data
        self.assert_rundata(last_line=999, last_offset=197326)
        self.assert_counters(critical_count=440, warning_count=28, ok_count=0, exec_count=0)

if __name__ == '__main__':
    unittest.main()


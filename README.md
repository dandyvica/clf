[![Build Status](https://www.travis-ci.com/dandyvica/clf.svg?branch=master)](https://www.travis-ci.com/dandyvica/clf)

# Reimplementation of the *check_logfiles* Nagios plugin

## Nagios check_logfiles
If you're familiar with Nagios plugins, you probably should be aware of its *check_logfiles* one. It allows to scan files (generally log files generated by UNIX daemons or Windows services) for different patterns. Each time a pattern is matched, an external script is called, along with arguments like regex captures groups.

The plug-in uses configuration files, which are plain *Perl* language regular files. These are imported by the *check_logfiles* script which uses them internally.

You can find the *check_logfiles* plugin here: https://labs.consol.de/nagios/check_logfiles/

## Implementation in Rust
While *Perl* is one of the fastest interpreted language, nothing can beat a compiled executable, specially when developed with a non-garbage collected one like C, C++ or Rust. When dealing with lots of servers in a professional environment, execution speed, memory footprint and scalability are paramount.

Rust is a relatively new system's programming language, providing speed, and security at compile time. For an introduction to Rust, please refer to https://www.rust-lang.org/ .

This reimplementation in Rust aims at solving original *check_logfiles* drawbacks:

* straightforward distribution: a single executable is needed. Using the *musl* static binary, no need for a specific verison of *libc*
* enhanced portability: same behaviour between Windows, Linux or BSD-like operating systems
* lightning speed: with an execution speed comparable to C or C++, execution time is not a hurdle
* ability to either call an external script or send JSON data coming from logfile to a TCP stream or a UNIX domain socket
* standard configuration file format: opposite to the original *check_logfiles* with uses non-standard configuration files (regular *Perl* files containing *Perl* variables), this implementation uses the YAML format for its configuration files. YAML is best suited comparing to JSON or XML because there's no need to escape chars for regexes expressions
* versatility: coupled with *Jinja2*-like well-known templates, you can imagine lots of possibilities to manage configuration files in a professionnal environment
* power: it will take into account not only regular log files, but also list of files command from a shell command or a script
* no need for a decompression binary: logfiles are *gunzipped* out of the box. Supported formats: gzip (extension: .gz), bzip2 (extension: .bz2), xz (extension: .xz)
* search for current or archived log files
* manage log rotations
* UTF-8-ready by default

*Caveat*: even though the current ```regex``` Rust crate is providing fast regexes checks, it doesn't currently support _lookahead or lookbehind_ patterns out of the box.

## Releases
Current release is 0.8 and currently still in a testing phase.

## Principle of operation
The *clf* executable processing is simple. After reading the YAML configuration file passed to the command line, it reads each logfile (or a list of logfiles provided by an external command or script) and tests each line against the defined regexes. If a match is found, it triggers an external script, either by spawning a new process and providing a set of environment variables to this process (and optionnally updating the *PATH* or *Path* variable, depending on the OS). Or by establishing a new socket connection to a remote address and port configured in the configuration file, and sending a JSON data structure with a set of variables coming from the search. Or even to a UNIX domain socket with the same principle.

The plugin output and exit code is depending on what is found in the provided logfiles.

## Format of the YAML configuration file
The current format of the configuration file defines where and what to search is a standard YAML format. 

Following is a list of current tags defined in the configuration file with a description of each tag:

```yaml
---
# a list of global options, valid for all searches.
global:
  # a list of ':'-separated list of paths (UNIX) or ';'-separated (Windows). If provided, the script is
  # searched within those directories.
  # Defaults to '/usr/sbin:/usr/bin:/sbin:/bin' or 'C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;' if not provided, depending on the 
  # platform.
  script_path: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

  # a path & file name where clf is keeping its runtime information. This is used to restart the search from the last
  # offset reached from the last clf run. If not provided, the file is the config file path with .json extension
  # could be overriden by the -d command line argument
  snapshot_file: /tmp/snapshot.json

  # retention time for tags in seconds. Defaults to 7 days
  snapshot_retention: 3600

  # a list of user variables, if any. Provided as-is to the callback (no CLF_ prefix)
  vars:
    first_name: Al
    last_name: Pacino
    city: 'Los Angeles'
    profession: actor

  # a list of executables or scripts started before searching into logfiles
  prescript:
    # command to run with its arguments
    - command: 
        - ./tests/integration/callbacks/echodomain.py
        - /tmp/echodomain.txt
        - /tmp/echodomain.sock

      # sleep current thread before continuing. In ms, defaults to 10
      timeout: 1000
  
      # async=true means clf will not wait for prescript succesfull execution
      async: true
  
      # stops clf processing if prescript return code is non 0
      exit_on_error: true

  # a command run at the end of logfiles processing. The list of pids from prescripts
  # is sent as arguments to this command
  postscript:
    command: ['./tests/integration/callbacks/kill.py']    
    timeout: 1000


# a list of logfiles & tags, to search for patterns. This is either a list of logfiles, or a command giving back a list of 
# files to search for.
searches:
  # logfile features
  - logfile: 
      # logfile path
      path: ./examples/logfiles/access_simple.log

      # format could be plain or JSON (JOSN to be developed)
      format: plain

      # lines matching the regex will be ignored
      exclude: ^#

      # specify logfile archive path and extension when rotated
      archive:
        # directory of archive. If not specified, current logfile directory
        dir: /tmp

        # archive extension
        extension: gz

      # what to report when a logfile is not found. Could be: critical, warning, unknown
      logfilemissing: critical

      # to determine whether a logfile has been rotated, inode & dev numbers might not be faithful. This is 
      # a buffer size which is used to calculate a CRC64 hash in case of inodes & devs are equal. Defaults to 4096
      hash_window: 2048


    # list of tags to refer to
    tags: 

      # tag name
      - name: http_access_get_or_post

        # set it to false to skip logfile processing. Defaults to true
        process: true

        # a list of comma-separated options to manage the search. See below for a list of options
        options: "runcallback"

        # a script or command to be called, every time a hit is found.
        callback: 
          script: ./tests/integration/callbacks/echovars.py
          args: 
            - /tmp/echovars.txt
            - arg2
            - arg3

        # list of patterns to match
        patterns:
          warning:
            # a list of regexes whic are considered as an error
            regexes:
              - 'GET\s+([/\w]+_logo\.jpg)'
              - 'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
            # if one of the exception regexes match, this is not considered as an error
            exceptions:
              - 'Firefox/63.0'
              - 'AppleWebKit/537\.36'


      # another tag for the same logfile
      - name: http_access_images
        options: "runcallback"
        callback: 
          domain: /tmp/echodomain.sock
          args: ['/tmp/echodomain.txt', 'arg2', 'arg3']
        patterns:
          critical:
            regexes: 
              - 'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
              - 'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
            exceptions:
              - 'AppleWebKit/537\.36'

     
  # this time, a list of files is given by executing this command and capturing its output.
  - logfile:
      list: ['find','/var/log', "-maxdepth", "2", "-type", "f", "-name", "[a-d]*.log"]
    tags: 
      - name: all_logs
        options: "runcallback"
        callback: 
          domain: /tmp/echodomain.sock
          args: ['/tmp/echodomain.txt', 'arg2', 'arg3']
        patterns:
          critical:
            regexes: 
              - 'error'
```

The way *clf* searches in logfiles is controlled, for each tag, by a list of comma-separated options. Some are only boolean options, soem others require to be set using an equality:

option | description
--- | ---
runcallback              | if set, the defined callback will be call for each line where a *critical* or *warning* pattern matches
rewind                   | if set, *clf* will read the considered logfile from the beginning, bypassing any offset recorded in the *snapshot* file
fastforward              | move to the end of the file, don't call any callback, if no snapshot data is found for the logfile
criticalthreshold=n  | when set to a 8-byte positive integer value, it means that critical errors will not be triggered unless this threshold is reached
warningthreshold=n   | when set to a 8-byte positive integer value, it means that warning errors will not be triggered unless this threshold is reached
savethresholds           | when set, either critical or warning threshold will be save in the *snapshot* file
runlimit=n           | when set, for each execution of *clf*, the defined script (if any) will only be called at most the value set by this option
truncate=n          | before matching any regex on a line, truncate the line to the specified number
runifok                 | if set, any defined callback is called even in case of an OK pattern found
stopat=n            | stop searching patterns when line number reaches the specified value
<br>
If a boolean option is not defined, it defaults to *false*. For integer options, they default to the maximum integer possible.

## Callback definition
A callback is either 
* a script which is called with environement variables depending on what is found during the search
* a TCP ip address to which data found are sent through a JSON string
* a UNIX domain socket (UNIX only) to which data found are sent through a JSON string

Examples of callbacks:

A script callback:
```yaml
callback: 
  script: ./tests/integration/callbacks/echovars.py
  args: ['/tmp/echovars.txt', 'arg2', 'arg3']
```
A TCP callback:
```yaml
callback: 
  address: 127.0.0.1:8999
  args: ['arg1', 'arg2', 'arg3']
```
A UNIX domain socket callback:
```yaml
callback: 
  domain: /tmp/clfdomain.sock
  args: ['arg1', 'arg2', 'arg3']
```

It's better to use the TCP or UDS callbacks because there's no overhead spawning an executable when matching lots of lines in a logfile. In case of a TCP or UDS callback, the receiving address or domain must be started before handling data from *clf*.

## Patterns definition
When *clf* fetches a line from a logfile, it compares this string with a list of *critical* regexes defined in the configuration file first, if any. Then it moves to comparing with a list of *warning* regexes if any. Ultimately, it compares with a list of *ok* patterns. This latter comparison makes *clf* to reset ongoing threshold to 0.

For *critical* or *warning* patterns, a list of exceptions can be defined, which invalidate any previous triggered match.

It is mandatory to use single quotes when using regexes, because it doesn't incur escaping characters. 

> Note: the current Rust *regex* crate doesn't support lookahead/lookbehind patterns. This can be alleviated using the *execptions* list, specially for negation regexes.

## Getting a list of files instead of a single one
Using the *list* YAML tag, it's possible to get a list of files. Following is an example for Windows & Linux:

```yaml
# example of a UNIX command, which returns a list of files
  - logfile:
      list: ['find','/var/log', "-maxdepth", "2", "-type", "f", "-name", "[a-d]*.log"]
```

```yaml
# example of a Windows command, which returns a list of files
  - logfile:
      list: ['cmd.exe', '/c', 'dir /B /S .\tests\integration\tmp\list_files.log.*']
```

## Data provided to the callback
Whenever a match is found when searching a logfile, if provided, a callback is called, with optional arguments. If the callback is a script, a list of environment variables is created and passed to the created process. If the callback is a TCP or UDS callback, all data are provided as a JSON string, with the JSON string length provided first. In case of a set of global variables, those are only provided during the first payload sent to the callback in case of a TCP or UDS callback, or each time in case of a script callback.

Following is the list of created variables:

variable name | description
---                                | --- 
CLF_LOGFILE                        | logfile name
CLF_CONFIG_FILE                    | configuration file name
CLF_HOSTNAME                       | machine hostname
CLF_PLATFORM                       | platform name
CLF_USER                           | user running *clf*
CLF_TAG                            | tag name
CLF_LINE                           | full line from the logfile, which triggered the match
CLF_LINE_NUMBER                    | the line number in the logfile, which triggered the match
CLF_MATCHED_RE                     | the regex (as a string) which triggered the match
CLF_MATCHED_RE_TYPE                | the type of regex which riggered the match (critical or warning)
CLF_CG_n                           | the value of the capture group involved in the match (n >= 0). Only in case of unnamed capture groups
CLF_NB_CG                          | number of capture groups
CLF_cgname                         | the value of the name capture group involved in the match
CLF_uservar1                       | the value of a user-defined variables defines in the *global:* YAML tag
CLF_OK_COUNT                       | current number of OK patterns found
CLF_WARNING_COUNT                  | current number of WARNING patterns found
CLF_CRITICAL_COUNT                 | current number of CRITICAL patterns found

<br>
You could easily gain access to those environment variables in scripting languages:

* Python: using the ```os.environ``` object, like this:

```python
import os
{ v:os.environ.get(v) for v in os.environ if v.startswith("CLF_") }

```

* Ruby: 

```ruby
ENV.select { |k,v| k.start_with?("CLF_") }
```

* bash:

```bash
#!/bin/bash
for v in ${!CLF_*}
do
    echo $v
done
```

## Domain socket callback example
You can an example for both a TCP or domain socket callback in the *examples* directory.

## Snapshot file
This is a JSON file where all state data are kept between different runs. Example:

```json
{
  "snapshot": {
    "logfiles/large_access.log": {
      "id": {
        "declared_path": "logfiles/large_access.log",
        "canon_path": "/data/projects/clf/examples/logfiles/large_access.log",
        "directory": "/data/projects/clf/examples/logfiles",
        "extension": "log",
        "compression": "uncompressed",
        "signature": {
          "inode": 1883537,
          "dev": 54,
          "size": 1108789736,
          "hash": 17022416761270139347
        }
      },
      "run_data": {
        "http_access_get_or_post": {
          "pid": 43570,
          "start_offset": 0,
          "start_line": 0,
          "last_offset": 1108789736,
          "last_line": 5412240,
          "last_run": "2021-02-03 19:21:52.647273302",
          "last_run_secs": 1612430512,
          "counters": {
            "critical_count": 97699,
            "warning_count": 7539,
            "ok_count": 0,
            "exec_count": 105238
          },
          "last_error": "None"
        }
      }
    }
  }
}
```

## List of command-line arguments
A self-explanatory help can be used with:

```console
clf --help
```
or

```console
clf -h
```

Following is a list of cli arguments:

```
USAGE:
    clf [FLAGS] [OPTIONS] --config <config>

FLAGS:
    -d, --delete-snapshot
            Delete snapshot file before searching

    -h, --help
            Prints help information

    -a, --no-callback
            Don't run any callback, just read all logfiles in the configuration file and print out
            matching line. Used to check whether regexes are correct

    -r, --overwrite-log
            Overwrite clf log if specified

    -o, --show-options
            Just show the command line options passed and exit

    -w, --show-rendered
            Render the configuration file through Jinja2/Tera and exit. This is meant to check Tera
            substitutions

    -s, --syntax-check
            Check configuration file correctness, print it out and exit

    -V, --version
            Prints version information


OPTIONS:
    -c, --config <config>
            Name of the YAML configuration file

    -x, --context <context>
            A JSON string used to set the Tera context. Only valid if the tera feature is enabled

    -l, --log <log>
            Name of the log file for logging information of this executable. Not to be confused with
            the logfile to search into

    -g, --log-level <log-level>
            When log is enabled, set the minimum log level. Defaults to 'Info'[possible values: Off,
            Error, Warn, Info, Debug, Trace]

    -m, --max-logsize <max-logsize>
            When log is enabled, set the maximum log size (in Mb). If specified, log file will be
            deleted first if current size is over this value. Defaults to 50 MB
            
    -p, --snapshot <snapshot>
            Override the snapshot file specified in the configuration file. It will default to the
            platform-dependent name using the temporary directory if not provided in configuration
            file or by using this flag

    -v, --var <var>...
            An optional variable to send to the defined callback, with syntax: 'var:value'. Multiple
            values are possible

```

## Plugin output
Here is an example of plugin output:

```
CRITICAL - (errors:662, warnings:28, unknowns:2)
tests/logfiles/small_access.log.gz: No such file or directory (os error 2)
/var/log/dpkg.log: OK - (errors:0, warnings:0, unknowns:0)
/var/log/auth.log: CRITICAL - (errors:146, warnings:0, unknowns:0)
/var/log/boot.log: Permission denied (os error 13)
tests/logfiles/small_access.log: CRITICAL - (errors:501, warnings:28, unknowns:0)
/var/log/bootstrap.log: CRITICAL - (errors:15, warnings:0, unknowns:0)
/var/log/alternatives.log: OK - (errors:0, warnings:0, unknowns:0)
```

## Compiling *clf*
First, clone the repository: 

```bash
$ git clone https://github.com/dandyvica/clf
```

Then, compile the package using the standard *cargo* command:

```bash
# the executable is:  ./target/release/clf or .\target\release\clf.exe
$ cargo build --release
```
To compile with the *musl* library as a standalone static executable:

```bash
# the executable is:  ./target/x86_64-unknown-linux-musl/release/clf
# or .\target\x86_64-unknown-linux-musl\release\clf.exe
$ cargo build --target x86_64-unknown-linux-musl --release   
```

Depending on your Linux distribution, you might need to install the *musl_tools*:
```bash
# install for Debian based distributions
$ sudo apt-get install musl-tools
```

## Windows specifics
In order to emulate UNIX inode/dev features, a specific DLL has been developed (*signature.dll*) You need to put this DLL in one of the paths specified by the Windows *Path* environment variable.

## Command line examples

```zsh
# mandatory argument: configuration file
$ clf --config config.yml

# delete snapshot first
$ clf --config config.yml --delete-snapshot

# use a specific snapshot file
$ clf --config config.yml --snapshot /tmp/temp_snapshot.json

# set the clf logger to a specific file
$ clf --config config.yml --log /tmp/clf.log

# don't run any callback, just output matching files for each tag
$ clf --config config.yml --no-callback

# check YAML syntax, print out internal representation and exit
$ clf --config config.yml --syntax-check

# show Tera/Jinaj2 rendered YAML and exit
$ clf --config config.yml --show-rendered

# add a global variable to any previously defined
$ clf --config config.yml --var "MY_VAR1:var1" "MY_VAR2:var2"

# override Tera context if the {{ path }} variable is not already set
$ clf --config config.yml --context '{ "path": "/var/sys/myapp.log" }'

# set log level
$ clf --config config.yml --log-level Trace
```

## References
* for a list of regex syntax: https://docs.rs/regex/1.4.3/regex/
* for Tera syntax: https://tera.netlify.app/docs/





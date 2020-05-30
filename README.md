# Reimplementation of the *check_logfiles* Nagios plugin

*Warning*: this is under construction and might change in the future.

## Nagios check_logfiles
If you're familiar with Nagios plugins, you probably should be aware of its *check_logfiles* one. It allows to scan files (generally log files generated by UNIX daemons or Windows services) for different patterns. Each time a pattern is matched, an external script is called, along with arguments like captures groups.

The plug-in uses configuration files, which are plain *Perl* language regular files. These are imported by the *check_logfiles* script and uses them internally.

You can find the *check_logfiles* plugin here: https://labs.consol.de/nagios/check_logfiles/

## Implementation in Rust
While *Perl* is one of the fastest interpreted language, nothing can beat a compiled executable, specially when developed with a non-garbage collected one like C, C++ or Rust. When dealing with lots of servers in a professional environment, execution speed, memory footprint and scalability are paramount.

Rust is a relatively new system's programming language, providing speed, and security at compile time. For an introduction to Rust, please refer to https://www.rust-lang.org/ .

This reimplementation in Rust aims at solving original *check_logfiles* drawbacks:

* straightforward distribution: a single executable is needed.
* enhanced portability: same behaviour between Windows, Linux or BSD-like operating systems.
* lightning speed: with an execution speed comparable to C or C++, execution time is not a hurdle. Multi-threaded or async/await is a future target.
* standard configuration file format: opposite to the original *cehck_logfiles* with uses non-standard files, this implementation will use YAML for its configuration files. YAML is best suited comparing to JSON or XML because there's no need to escape chars for regexes expressions.
* versatility: coupled with *Jinja2*-like well-known templates, you can imagine lots of possibilities to manage configuration files in a professionnal environment.
* power: it will take into account not only regular log files, but also list of files command from a shell command or a script.
* no need for a decompression binary: logfiles are *gunzipped* out of the box.
* search for current or archived log files.

*Caveat*: Even though the current ```regex``` Rust crate is providing fast regexes checks, it doesn't currently support lookahead/behind patterns.

## Releases
Current release is 0.1 and currently in development. It should be considered as bleeding edge or pre-α stage.

## Format of the YAML configuration file
The current format of the configuration file needed to define where and what to search is a standard YAML format. 

Example of a YAML configuration file, to search for patterns in 

Following is a list of current tags defined in the configuration file:

```yaml
---
# a list of global options, valid for all searches.
global:
  # a list of ':'-separated list of paths (UNIX) or ';'-separated (Windows). If provided, the script is
  # searched within those directories.
  # Defaults to '/usr/sbin:/usr/bin:/sbin:/bin' or 'C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;' if not provided, depending on the 
  # platform.
  path: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

  # a path & file name where clf is keeping its runtime information. This is used to restart the search from the last
  # offset reached from the last clf run.
  snapshot_file: /tmp/snapshot.json

  # retention time for tags in seconds
  snapshot_retention: 5

# a list of logfiles & tags, to search for patterns. This is either a list of logfiles, or a command giving back a list of 
# files to search for.
searches:
  # name & path of the logfile to search
  - logfile: tests/logfiles/large_access.log

    # list of tags to refer to
    tags: 

      # tag name
      - name: http_access

        # a list of comma-separated options to manage the search. Current supported options are:
        # runscript: if present, means the provided script will be called
        # rewind: restart the search from the beginning of the file
        options: "runscript,"

        # a script or command to be called, every time a hit is found.
        script: { 
          path: "tests/scripts/echovars.py", 
          args: ['arg1', 'arg2', 'arg3']
        }

        # list of patterns to match
        patterns:
          critical: {
            regexes: [
              'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
              'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
            ],
            exceptions: [
              'AppleWebKit/537\.36'
            ]
          }

      
  # name & path of the logfile to search. This file is gzipped.
  - logfile: tests/logfiles/large_access.log.gz
    tags: 
      - name: http_access_gzipped
        options: "runscript,"
        script: { 
          path: "tests/scripts/echovars.py", 
          args: ['arg1', 'arg2', 'arg3']
        }
        patterns:
          critical: {
            regexes: [
              'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
              'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
            ],
            exceptions: [
              'AppleWebKit/537\.36'
            ]
          }

  # this time, a list of files is given by executing this command and capturing its output.
  - loglist: { 
      cmd: 'find',
      args: ['/var/log', "-maxdepth", "1", "-type", "f"]
    }
    tags: 
      - name: all_logs
        options: "runscript,"
        script: { 
          path: "tests/scripts/echovars.py", 
          args: ['arg1', 'arg2', 'arg3']
        }
        patterns:
          critical: {
            regexes: [
              'error'
            ]
          }
```

## Environment provided to the called scripts or commands
Whenever a match is found when searching a logfile, if provided, as script is called, with optional arguments. A list of environment variables is created and passed to the created process, to be used by the called script. Following is the list of created variables:

variable name | description
---                                | --- 
CLF_LOGFILE                        | logfile name
CLF_TAG                            | tag name
CLF_LINE                           | full line from the logfile, which triggered the match
CLF_LINE_NUMBER                    | the line number in the logfile, which triggered the match
CLF_MATCHED_RE                     | the regex (as a string) which triggered the match
CLF_MATCHED_RE_TYPE                | the type of regex which riggered the match (critical or warning)
CLF_CAPTUREn                       | the value of the capture group involved in the match (n >= 0). Only in case of unnamed capture groups
CLF_cgname                         | the value of the name capture group involved in the match
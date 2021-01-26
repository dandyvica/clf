#!/usr/bin/perl

# just fetch the relevant environment variable from arguments
# and create a logfile
use strict;
use warnings;
 
 # create the file from fetch env variable
my $filename = '/tmp/concatenated.log.sh';
open(my $fh, '>', $filename) or die "Could not open file '$filename' $!";
print $ENV{'my_awesomescript'};
print $fh $ENV{'my_awesomescript'};
close $fh;

# now just run file
chmod 0744, $filename;

# run it
my $exit_code = system($filename);
exit($exit_code);




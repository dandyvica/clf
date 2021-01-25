#!/usr/bin/perl

# just fetch the relevant environment variable from arguments
# and create a logfile
use strict;
use warnings;
 
 # create the file from fetch env variable
my $filename = '/tmp/concatenated.log';
open(my $fh, '>', $filename) or die "Could not open file '$filename' $!";
print $fh $ENV{my_awesomescript};
close $fh;



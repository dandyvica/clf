#!/usr/bin/python3
# generate a dummy logfile for intergation tests
import sys
import argparse
from random import randrange, randint
from datetime import timedelta, datetime

def generate_random_date(start, end):
    """
    This function will return a random datetime between two datetime 
    objects.
    """
    delta = end - start
    int_delta = (delta.days * 24 * 60 * 60) + delta.seconds
    random_second = randrange(int_delta)
    return start + timedelta(seconds=random_second)

# constants
ERROR_LINE = "{random_date}: this is an error generated for tests, line number = {line_number}, error id = {random_error}\n"
WARNING_LINE = "{random_date}: this is a warning generated for tests, line number = {line_number}, warning id = {random_error}\n"
OK_LINE = "{random_date}: this is a fake ok pattern at line {line_number}\n"
START_DATE = datetime.strptime('1/1/2008 1:30 PM', '%m/%d/%Y %I:%M %p')
END_DATE = datetime.strptime('1/1/2020 4:50 AM', '%m/%d/%Y %I:%M %p')

def generate_lines(n: int, output: str, append: bool):
    """ generate a fake logfile with n lines """
    mode = "w" if not append else "a"
    line_number = 0

    # create file according to mode
    f = open(output, mode)

    for i in range(1,n+1):
        line_number += 1
        random_error = randint(10000, 99999)
        random_date = generate_random_date(START_DATE, END_DATE)

        if i == (n+1)/2:
            f.write(OK_LINE.format(random_date=random_date, line_number=line_number))
            continue

        f.write(ERROR_LINE.format(random_date=random_date, line_number=line_number, random_error=random_error))
        line_number += 1
        f.write(WARNING_LINE.format(random_date=random_date, line_number=line_number, random_error=random_error))


# manage arguments
parser = argparse.ArgumentParser()
parser.add_argument("--n", help="number of lines of the generated file", required=True, type=int)
parser.add_argument("--output", help="name of the output file", required=True, type=str)
parser.add_argument("--append", help="if specified, additional lines will be added to the output file (append mode)", required=False, action='store_true')
args = parser.parse_args()

# manage only even n
if args.n % 2 == 0:
    print("please enter an even number for n")
    sys.exit(1)

# number of lines as 1st argument
generate_lines(args.n, args.output, args.append)


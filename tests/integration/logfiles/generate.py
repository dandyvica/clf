#!/usr/bin/python3
# generate a dummy logfile for intergation tests
import sys
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

# number of lines as 1st argument
nb_lines = int(sys.argv[1])
if nb_lines % 2 == 0:
    print("specify an odd number")
    sys.exit(1)

# to generate dummy dates
d1 = datetime.strptime('1/1/2008 1:30 PM', '%m/%d/%Y %I:%M %p')
d2 = datetime.strptime('1/1/2020 4:50 AM', '%m/%d/%Y %I:%M %p')
line_number = 0

for i in range(1,nb_lines+1):
    line_number += 1
    random_error = randint(10000, 99999)
    random_date = generate_random_date(d1, d2)

    if i == (nb_lines+1)/2:
        print(f"{random_date}: this is a fake ok pattern at line {line_number}")
        continue    

    print(f'{random_date}: this is an error generated for tests, line number = {line_number}, error id = {random_error}')
    line_number += 1
    print(f'{random_date}: this is a warning generated for tests, line number = {line_number}, warning id = {random_error}')


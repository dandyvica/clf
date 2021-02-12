#!/usr/bin/env python
""" save all received data from clf into a file """

import socket
import sys
import json
import os
import argparse

#--------------------------------------------------------------------------------------------------
# manage cli arguments
#--------------------------------------------------------------------------------------------------
parser = argparse.ArgumentParser(
        description="echo or save json data coming from clf", 
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

parser.add_argument("--domain", type=str, help="name and path of the domain socket", default="echodomain.sock")
parser.add_argument("--output", type=str, help="if specified, save JSON data into this file", default=None)
args = parser.parse_args()


# file name to save input data
server_address = args.domain
if args.output is not None:
    fh = open(args.output, "w")
else:
    fh = sys.stdout

# Make sure the socket does not already exist
try:
    os.unlink(server_address)
except OSError:
    if os.path.exists(server_address):
        raise

# Create a UDS socket
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)

# bind
fh.write("starting up for domain address %s \n" % server_address)
sock.bind(server_address)

# Listen for incoming connections
sock.listen(1)

while True:
    # Wait for a connection
    fh.write('waiting for a connection\n')
    try:
        connection, client_address = sock.accept()

        # number of JSON data received so far
        nb_json = 0

        try:
            fh.write('connection from %s\n' % client_address)

            # Receive the data in small chunks and retransmit it
            while True:
                # receive JSON size in network order (big endian)
                data = connection.recv(2)
                if not data:
                    fh.write("end of data\n")
                    break
                
                # first receive JSON data size
                json_size = int(data.encode('hex'), 16)
                fh.write("received size = %d\n" % json_size)

                # receive JSON payload
                json_data = connection.recv(json_size)
                if not json_data:
                    fh.write("end of data\n")
                    break

                # decode and display JSON
                nb_json += 1
                decode = json_data.decode("ascii", errors="ignore")
                parsed = json.loads(decode)

                # test if we were told to end
                if "terminate" in parsed and parsed["terminate"] is True:
                    fh.write("terminating...\n")
                    connection.close()
                    sys.exit(0)

                # otherwise write data into output file
                pretty = json.dumps(parsed, indent=4, sort_keys=False)
                fh.write("JSON#: %d, received data: %s\n" % (nb_json, pretty))
                    
        finally:
            # Clean up the connection
            fh.write("close connection\n")
            connection.close                

    except KeyboardInterrupt:
        fh.write("KeyboardInterrupt received\n")
        sys.exit(1)
      

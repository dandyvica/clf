#!/usr/bin/env python
import socket
import sys
import json
import os

# file name to save input data
server_address = sys.argv[1]
output_file = sys.argv[2]

# open file for writing
f = open(output_file, "w")

# Make sure the socket does not already exist
try:
    os.unlink(server_address)
except OSError:
    if os.path.exists(server_address):
        raise

# Create a UDS socket
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)

# bind
f.write("starting up for domain address %s \n" % server_address)
sock.bind(server_address)

# Listen for incoming connections
sock.listen(1)

while True:
    # Wait for a connection
    f.write('waiting for a connection\n')
    connection, client_address = sock.accept()

    # number of JSON data received so far
    nb_json = 0

    try:
        f.write('connection from %s\n' % client_address)

        # Receive the data in small chunks and retransmit it
        while True:
            # receive JSON size in network order (big endian)
            data = connection.recv(2)
            if not data:
                f.write("end of data\n")
                break
            
            # first receive JSON data size
            json_size = int(data.encode('hex'), 16)
            f.write("received size = %d\n" % json_size)

            # receive JSON payload
            json_data = connection.recv(json_size)
            if not json_data:
                f.write("end of data\n")
                break

            # decode and display JSON
            nb_json += 1
            decode = json_data.decode("ascii", errors="ignore")
            parsed = json.loads(decode)

            # test if we were told to end
            if "terminate" in parsed and parsed["terminate"] is True:
                f.write("terminating...\n")
                connection.close()
                sys.exit(0)

            # otherwise write data into output file
            pretty = json.dumps(parsed, indent=4, sort_keys=False)
            f.write("JSON#: %d, received data: %s\n" % (nb_json, pretty))
            
    finally:
        # Clean up the connection
        f.write("close connection\n")
        connection.close()


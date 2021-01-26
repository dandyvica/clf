#!/usr/bin/env python3
import socket
import sys
import json
import struct
import os

# file name to save input data
saved = sys.argv[1]
f = open(saved, "w")


# UDS name might be passed as an argument
if len(sys.argv) == 2:
    server_address = '/tmp/clf.sock'
else:
    server_address = sys.argv[2]

# Make sure the socket does not already exist
try:
    os.unlink(server_address)
except OSError:
    if os.path.exists(server_address):
        raise

# Create a UDS socket
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)

# bind
print('starting up for domain address %s ' % server_address)
sock.bind(server_address)

# Listen for incoming connections
sock.listen(1)

while True:
    # Wait for a connection
    print('waiting for a connection')
    connection, client_address = sock.accept()

    # number of JSON data received so far
    nb_json = 0

    try:
        print('connection from', client_address)

        # Receive the data in small chunks and retransmit it
        while True:
            # receive JSON size in network order (big endian)
            data = connection.recv(2)
            if not data:
                f.write("end of data\n")
                break
            
            # first receive JSON data size
            json_size = int.from_bytes(data, byteorder='big')
            f.write(f"received size = {json_size}\n")

            # receive JSON payload
            json_data = connection.recv(json_size)
            if not json_data:
                f.write("end of data\n")
                break

            # decode and display JSON
            nb_json += 1
            decode = json_data.decode("ascii", errors="ignore")
            parsed = json.loads(decode)
            pretty = json.dumps(parsed, indent=4, sort_keys=False)
            f.write(f"JSON#: {nb_json}, received data: {pretty}\n")
            
    finally:
        # Clean up the connection
        f.write("close connection\n")
        connection.close()


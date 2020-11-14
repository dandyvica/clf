#!/usr/bin/python3
import socket
import sys
import json
import struct
import os

# UDS name
server_address = '/tmp/clf.sock'

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
                break
            size = int.from_bytes(data, byteorder='big')
            #print("size==", size)

            # receive JSON payload
            data = connection.recv(size)
            if not data:
                break

            # decode and display JSON
            nb_json += 1
            decode = data.decode("ascii", errors="ignore")
            parsed = json.loads(decode)
            pretty = json.dumps(parsed, indent=4, sort_keys=False)
            print('JSON# %s, received "%s"' % (nb_json, pretty))
            
    finally:
        # Clean up the connection
        connection.close()


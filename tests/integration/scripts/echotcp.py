#!/usr/bin/python3
import socket
import sys
import json
import struct

# Create a TCP/IP socket
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

# use this to made the socket immediatly available
#sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

# Bind the socket to the port
server_address = ('localhost', 8999)
print('starting up on %s port %s' % server_address)
sock.bind(server_address)

# Listen for incoming connections
sock.listen(1)

while True:
    # Wait for a connection
    print('waiting for a connection')
    connection, client_address = sock.accept()

    try:
        print('connection from', client_address)

        # Receive the data in small chunks and retransmit it
        while True:
            # receive JSON size in network order (big endian)
            data = connection.recv(2)
            if not data:
                break
            size = int.from_bytes(data, byteorder='big')
            print("size==", size)

            # receive JSON payload
            data = connection.recv(size)
            if not data:
                break

            # decode and display JSON
            decode = data.decode("ascii", errors="ignore")
            parsed = json.loads(decode)
            pretty = json.dumps(parsed, indent=4, sort_keys=False)
            print('received "%s"' % pretty)
            
    finally:
        # Clean up the connection
        connection.close()


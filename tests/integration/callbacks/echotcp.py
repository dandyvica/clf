#!/usr/bin/env python2.7
import socket
import sys
import json
import struct

# file name to save input data
server_address = sys.argv[1].split(":")
addr = server_address[0]
port = int(server_address[1])
output_file = sys.argv[2]

# open file for writing
f = open(output_file, "w")

# Create a TCP/IP socket
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

# use this to made the socket immediatly available
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

# Bind the socket to the port
f.write("starting up for TCP address %s:%s \n" % (addr,port))
sock.bind((addr,port))

# Listen for incoming connections
sock.listen(1)

while True:
    # Wait for a connection
    f.write('waiting for a connection\n')
    connection, client_address = sock.accept()

    # number of JSON data received so far
    nb_json = 0

    try:
        f.write('connection from %s\n' % (client_address,))

        # Receive the data in small chunks and retransmit it
        while True:
            # receive JSON size in network order (big endian)
            data = connection.recv(2)
            if not data:
                break
            json_size = int(data.encode('hex'), 16)
            f.write("received size = %d\n" % json_size)

            # receive JSON payload
            data = connection.recv(json_size)
            if not data:
                break

            # decode and display JSON
            decode = data.decode("ascii", errors="ignore")
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


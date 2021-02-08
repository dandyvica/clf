#!/usr/bin/env python
""" this script is gracefully terminating socket connexion """

import socket
import sys
import json
import struct

# file name to save input data
socket_or_domain = sys.argv[2]

# Create and bind to a socket
if socket_or_domain == "domain":
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server_address = sys.argv[1]
    sock.connect(server_address)
else:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_address = sys.argv[1].split(":")
    addr = server_address[0]
    port = int(server_address[1])
    sock.connect((addr,port))

# send termination string
data = {
    'terminate' : True
}
json_data = json.dumps(data)

# send data: length + payload
data_size = len(json_data.encode('utf-8'))
sock.sendall(struct.pack('!H', data_size))
sock.sendall(json_data)

# close socket
sock.close()





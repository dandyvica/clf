#!/usr/bin/ruby
require 'socket'      # Sockets are in standard library

server = TCPServer.new(8999)
begin
    while connection = server.accept
        while line = connection.gets
            break if line =~ /quit/
            puts line
            connection.puts "Received!\n"
        end
        connection.puts "Closing the connection. Bye!\n"
        connection.close
        #server.close
    end
rescue Errno::ECONNRESET, Errno::EPIPE => e
    puts e.message
    retry
end    
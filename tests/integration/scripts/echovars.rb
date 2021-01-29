# first arg is the name of the output file
output_file = ARGV[0]

# get list of variables for CLF
clf = ENV.select {|k,v| k.start_with?("CLF_")}
pid = Process.pid

# build file into temporary directory with the first argument as the file name
output = File.open(output_file, "a")
output.write("#{pid}-#{output_file}-#{clf}\n")

exit(100)
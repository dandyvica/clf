# a simple class for storing and executing testcases
require 'yaml'
require 'json'

# a class for running all testcases
# class TestScenario
#     attr_reader :root, :clf, :yaml

#     # load and keep YAML internally
#     def initialize(yaml_file)
#         @yaml = YAML.load_file(yaml_file)

#         # save paths for executing clf
#         @root = @yaml["setup"]["root"]
#         @clf_path = @yaml["setup"]["clf"]
#     end

#     # run each testcase
#     def run
#         # iterating over the array of hash testcases. tc is a hash with a single key
#         @yaml["testcases"].each do |tc|
#             # tag is the first and only key
#             tag = tc.keys[0]
#             puts "running tc #{tag}"

#             # extract data
#             tc_data = tc[tag]

#             # build new test case
#             puts tc_data
#             test_case = TestCase.new(@clf_path, @root, tc_data)
#         end
#     end

# end

# a class for one testcase
class TestCase
    # input is a hash defining the testcase
    # hask key is the tc tag
    attr_reader :config_file

    def initialize(tag, root, clf_path)
        @tag = tag
        @clf = clf_path

        @snap_file = File.join(root, "tmp", "#{tag}.json")
        @config_file = File.join(root, "tmp", "#{tag}.yml")
        @logfile = File.join(root, "tmp", "#{tag}.log")
        @logfile_gzip = File.join(root, "tmp", "#{tag}.log.gz")
        @tmpfile = File.join(root, "tmp", "#{tag}.txt")

        # cleanup previous files
        File.delete(@logfile) if File.exist?(@logfile)
        File.delete(@logfile_gzip) if File.exist?(@logfile_gzip)
        File.delete(@snap_file) if File.exist?(@snap_file)
        File.delete(@tmpfile) if File.exist?(@tmpfile)

        puts "executing test case: #{@tag}"
    end

    def create_log(append=false)
        FakeLogfile.create(@logfile)
    end

    # run the test case using the executable argument
    def run(opts="")
        args = "--config #{@config_file} --snapshot #{@snap_file} #{opts}"
        self.exec(args)

        # read JSON in this case
        @snapshot = JSON.parse(File.read(@snap_file))
    end

    def exec(args)
        cmd = "#{@clf} #{args}"
        @output = `#{cmd}`
        @rc = $?
    end


end

# creates fake logfiles
class FakeLogfile
    def self.create(path, append=false)
        # open for writing or appending 
        file = append ? File.open(path, "a") : File.open(path, "w")

        # write n lines
        n = 0
        101.times do
            n += 1
            line = "%03d" % n

            if n == 51 then
                file.puts "1970-01-01 00:00:00: ############# this is a fake ok pattern generated for tests, line number = #{line}"
                next
            end

            random = rand(10000..99999)
            file.puts "1970-01-01 00:00:00: ---- this is an error generated for tests, line number = #{line}, error id = #{random}"

            n += 1
            line = "%03d" % n
            random = rand(10000..99999)
            file.puts "1970-01-01 00:00:00: * this is an warning generated for tests,line number = #{line}, warning id = #{random}"
        end

        file.close

        # check size
        raise "bad logfile size" unless File.new(path).size == 20100
    end
end

# a class for managing creating of YAML config file
class Config
    def initialize(yaml_file)
        @yaml = YAML.load_file(yaml_file)
        pp @yaml
    end


end


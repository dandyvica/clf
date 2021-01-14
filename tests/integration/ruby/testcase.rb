# a simple class for storing and executing testcases
require 'yaml'

# a class for running all testcases
class TestScenario
    attr_reader :root, :clf, :yaml

    # load and keep YAML internally
    def initialize(yaml_file)
        @yaml = YAML.load_file(yaml_file)

        # save paths for executing clf
        @test_dir = @yaml["setup"]["test_dir"]
        @clf_path = @yaml["setup"]["clf"]
    end

    # run each testcase
    def run
        # iterating over the array of hash testcases. tc is a hash with a single key
        @yaml["testcases"].each do |tc|
            # tag is the first and only key
            tag = tc.keys[0]
            puts "running tc #{tag}"

            # extract data
            tc_data = tc[tag]

            # build new test case
            puts tc_data
            test_case = TestCase.new(@clf_path, @test_dir, tc_data)
        end
    end

end

# a class for one testcase
class TestCase
    # input is a hash defining the testcase
    # hask key is the tc tag
    def initialize(clf_path, test_dir, tc_data)
        @clf_path = clf_path
        @name = tc_data["name"]
        @config_file = File.join(test_dir, "tmp", "#{name}.yml")
        @snap_file = File.join(test_dir, "tmp", "#{name}.json")



        puts "executing test case: #{@name}"
    end

    # run the test case using the executable argument
    def run(clf)
        args = "--config #{@config_file} --snapshot #{@snap_file}"
        cmd = "#{clf_path} #{args}"
        output = ``
    end
end


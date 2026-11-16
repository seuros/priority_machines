# frozen_string_literal: true

require 'minitest/autorun'
require 'open3'
require 'rbconfig'

# The classes of the schedulers the extension wraps (RoundRobin,
# WeightedRoundRobin, DeficitRoundRobin) are fixed when it loads. Nothing a
# caller does later, redefining a constant or `Object.const_get`, reaches a
# scheduler being built. Each case runs in a fresh process: the class is fixed
# once per process, at load.
class ClassLookupTest < Minitest::Test
  LIB = File.expand_path('../lib', __dir__)

  PRELUDE = <<~RUBY
    require 'priority_machines'
    PM = PriorityMachines
  RUBY

  # Runs `body` after PRELUDE in a new Ruby; answers its last line of output.
  def isolated(body)
    out, err, status = Open3.capture3(RbConfig.ruby, '-I', LIB, '-e', PRELUDE + body)
    assert status.success?, "exited #{status.exitstatus}: #{err}"
    out.lines.last&.chomp
  end

  # Bug: a scheduler constant redefined before the first `new` made the value
  # an instance of the new class, without #select.
  def test_a_redefined_constant_does_not_change_the_scheduler_class
    assert_equal 'true 0 1|true 0 0|true 0 0', isolated(<<~RUBY)
      built = { RoundRobin: [[true, true]], WeightedRoundRobin: [[1, 1], [true, true]],
                DeficitRoundRobin: [[1, 1], [1, 1]] }.map do |name, (*args, view)|
        original = PM.const_get(name)
        PM.send(:remove_const, name)
        PM.const_set(name, Class.new)
        scheduler = original.new(*args)
        [scheduler.instance_of?(original), scheduler.select(view),
         name == :RoundRobin ? scheduler.cursor : 0].join(' ')
      end
      puts built.join('|')
    RUBY
  end

  # Bug: building the first scheduler ran `Object.const_get`, Ruby code in
  # the middle of a native constructor.
  def test_building_a_scheduler_runs_no_const_get
    assert_equal '0 RoundRobin WeightedRoundRobin DeficitRoundRobin', isolated(<<~RUBY)
      hits = 0
      Object.define_singleton_method(:const_get) do |*args|
        hits += 1
        super(*args)
      end
      built = [PM::RoundRobin.new, PM::WeightedRoundRobin.new([1]), PM::DeficitRoundRobin.new([1])]
      puts [hits, *built.map { |s| s.class.name.split('::').last }].join(' ')
    RUBY
  end

  # Bug: a redefined `Object.const_get` that raised hit an `unwrap` in the
  # extension: an uncatchable fatal, and the process exited.
  def test_a_raising_const_get_does_not_kill_the_process
    assert_equal '0 0 0', isolated(<<~RUBY)
      Object.define_singleton_method(:const_get) { |*| raise IOError, 'lookup ran in a constructor' }
      built = [PM::RoundRobin.new.select([true]), PM::WeightedRoundRobin.new([1]).select([true]),
               PM::DeficitRoundRobin.new([1]).select([1])]
      puts built.join(' ')
    RUBY
  end
end

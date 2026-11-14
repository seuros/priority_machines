# frozen_string_literal: true

require 'minitest/autorun'
require 'priority_machines'

class PriorityMachinesTest < Minitest::Test
  PM = PriorityMachines

  def test_weighted_reference_and_exact_lane_counts
    (0..16).each do |n|
      scheduler = PM::WeightedRoundRobin.new((1..n).to_a)
      expected = (0...n).flat_map { |i| [i] * (i + 1) }
      assert_equal expected, expected.map { scheduler.select([true] * n) }
      assert_nil scheduler.select([false] * n)
      assert_equal :unknown_lane, scheduler.deactivate(n)
    end
  end

  def test_deficit_reference
    scheduler = PM::DeficitRoundRobin.new([3, 1])
    assert_equal [0, 0, 0, 1] * 5, Array.new(20) { scheduler.select([1, 1]) }
    assert_nil scheduler.select([nil, nil])
    assert_equal 0, scheduler.select([4_294_967_295, nil])
    assert_nil PM::DeficitRoundRobin.new([]).select([])
  end

  def test_deactivate_answers_every_outcome_as_a_symbol
    { PM::WeightedRoundRobin.new([3, 1]) => [true, true], PM::DeficitRoundRobin.new([3, 1]) => [1, 1] }
      .each do |scheduler, view|
        assert_equal 0, scheduler.select(view)
        assert_equal :deactivated, scheduler.deactivate(0)
        assert_equal :idle, scheduler.deactivate(0)
        [2, 99, (2**64) - 1].each { |lane| assert_equal :unknown_lane, scheduler.deactivate(lane) }
      end
  end

  def test_deactivating_lanes_that_hold_nothing_keeps_every_lane_in_place
    weighted = PM::WeightedRoundRobin.new([3, 2])
    deficit = PM::DeficitRoundRobin.new([3, 2])
    [0, 0, 1, 0, 1].each do |lane|
      assert_equal :idle, weighted.deactivate(lane)
      assert_equal :idle, deficit.deactivate(lane)
    end
    assert_equal [0, 0, 0, 1, 1], Array.new(5) { weighted.select([true, true]) }
    assert_equal [0, 0, 0, 1, 1], Array.new(5) { deficit.select([1, 1]) }
  end

  def test_refused_and_repeated_deactivations_change_nothing
    twins = Array.new(2) { PM::WeightedRoundRobin.new([3, 2]) }
    twins.each { |scheduler| assert_equal 0, scheduler.select([true, true]) }
    assert_equal :unknown_lane, twins.first.deactivate(2)
    assert_equal :idle, twins.first.deactivate(1)
    assert_equal(*twins.map { |scheduler| Array.new(10) { scheduler.select([true, true]) } })

    twins = Array.new(2) { PM::DeficitRoundRobin.new([5, 2]) }
    twins.each { |scheduler| assert_equal 0, scheduler.select([2, 2]) }
    assert_equal :unknown_lane, twins.first.deactivate(2)
    assert_equal :idle, twins.first.deactivate(1)
    assert_equal(*twins.map { |scheduler| Array.new(10) { scheduler.select([2, 2]) } })
  end

  def test_an_empty_deficit_round_clears_credit_and_keeps_the_cursor
    scheduler = PM::DeficitRoundRobin.new([4, 9])
    assert_equal [0, 1], Array.new(2) { scheduler.select([3, 3]) }
    2.times { assert_nil scheduler.select([nil, nil]) }
    # Lane 1 is still next and is paid a fresh quantum: three heads, not five.
    assert_equal [1, 1, 1, 0], Array.new(4) { scheduler.select([3, 3]) }
  end

  def test_round_robin_mirrors_the_core
    scheduler = PM::RoundRobin.new
    assert_equal 0, scheduler.cursor
    assert_equal [1, 2], Array.new(2) { scheduler.select([false, true, true]) }
    assert_equal 0, scheduler.select([true, true])
    assert_equal 1, scheduler.cursor
    assert_nil scheduler.select([false, false])
    assert_nil scheduler.select([])
    assert_equal 1, scheduler.cursor
    assert_equal [1, 0], [scheduler.select([true] * 2), scheduler.select([true] * 5)]
  end

  def test_round_robin_validates_the_whole_call_first
    scheduler = PM::RoundRobin.new
    assert_equal 0, scheduler.select([true, true])
    [nil, 1, 'true'].each { |flag| assert_raises(TypeError) { scheduler.select([true, flag]) } }
    assert_raises(TypeError) { scheduler.select(true) }
    assert_equal 1, scheduler.cursor
    assert_raises(ArgumentError) { PM::RoundRobin.new(3) }
    scheduler.freeze
    assert_raises(FrozenError) { scheduler.select([true]) }
    assert_raises(FrozenError) { scheduler.select(nil) }
    assert_raises(FrozenError) { scheduler.select([nil]) }
    assert_equal 1, scheduler.cursor
  end

  def test_array_subclasses_are_refused
    flags = Class.new(Array)
    error = assert_raises(TypeError) { PM::RoundRobin.new.select(flags[true]) }
    assert_equal 'ready must be an Array', error.message
    error = assert_raises(TypeError) { PM.earliest_deadline(flags[1]) }
    assert_equal 'deadlines must be an Array', error.message
    error = assert_raises(TypeError) { PM::WeightedRoundRobin.new(flags[1]) }
    assert_equal 'weights must be an Array', error.message
    error = assert_raises(TypeError) { PM::DeficitRoundRobin.new(flags[1]) }
    assert_equal 'quanta must be an Array', error.message
    error = assert_raises(TypeError) { PM::WeightedRoundRobin.new([1]).select(flags[true]) }
    assert_equal 'ready must be an Array', error.message
    deficit = PM::DeficitRoundRobin.new([1])
    error = assert_raises(TypeError) { deficit.select(flags[1]) }
    assert_equal 'heads must be an Array', error.message
    assert_equal 0, deficit.select([1])
  end

  def test_earliest_deadline_takes_integer_nanoseconds
    assert_nil PM.earliest_deadline([])
    assert_nil PM.earliest_deadline([nil, nil])
    assert_equal 1, PM.earliest_deadline([nil, 1, 1])
    assert_equal 2, PM.earliest_deadline([10_000_000_000, nil, 250_000_000])
    assert_equal 1, PM.earliest_deadline([1_500_000_000, 1_499_999_999])
    assert_equal 1, PM.earliest_deadline([(2**64) - 1, (2**64) - 2])
    assert_equal 0, PM.earliest_deadline([0, 0])
  end

  def test_deadlines_are_never_floats_or_seconds
    [1.0, 0.25, Float::NAN, Float::INFINITY, -0.1, '1', true, Object.new].each do |value|
      assert_raises(TypeError) { PM.earliest_deadline([value]) }
    end
    [-1, 2**64].each do |value|
      assert_raises(RangeError) { PM.earliest_deadline([value]) }
    end
    assert_raises(TypeError) { PM.earliest_deadline(nil) }
  end

  def test_entire_call_validated_before_mutation
    weighted = PM::WeightedRoundRobin.new([2, 1])
    assert_raises(TypeError) { weighted.select([true, Object.new]) }
    assert_raises(TypeError) { weighted.select([true, nil]) }
    assert_raises(RangeError) { weighted.select([true]) }
    assert_equal [0, 0, 1], Array.new(3) { weighted.select([true, true]) }
    deficit = PM::DeficitRoundRobin.new([2, 1])
    assert_raises(TypeError) { deficit.select([1, 1.5]) }
    assert_raises(RangeError) { deficit.select([1, 0]) }
    assert_raises(RangeError) { deficit.select([1, 1, 1]) }
    assert_equal [0, 0, 1], Array.new(3) { deficit.select([1, 1]) }
  end

  def test_numeric_boundaries
    assert_equal 16, PM::MAX_LANES
    error = assert_raises(RangeError) { PM::WeightedRoundRobin.new([1] * (PM::MAX_LANES + 1)) }
    assert_equal 'weights holds at most 16 lanes', error.message
    assert_raises(RangeError) { PM::DeficitRoundRobin.new([1] * (PM::MAX_LANES + 1)) }
    [1.5, '1', nil, Object.new].each do |value|
      assert_raises(TypeError) { PM::DeficitRoundRobin.new([value]) }
      assert_raises(TypeError) { PM::WeightedRoundRobin.new([value]) }
    end
    [0, -1, 2**32].each do |value|
      assert_raises(RangeError) { PM::WeightedRoundRobin.new([value]) }
      assert_raises(RangeError) { PM::DeficitRoundRobin.new([value]) }
    end
  end

  def test_constructors_are_positional_like_the_core
    assert_raises(ArgumentError) { PM::WeightedRoundRobin.new }
    assert_raises(ArgumentError) { PM::DeficitRoundRobin.new }
    assert_raises(TypeError) { PM::WeightedRoundRobin.new(weights: [1]) }
    assert_raises(TypeError) { PM::DeficitRoundRobin.new(quanta: [1]) }
  end

  def test_messages_name_the_argument
    error = assert_raises(TypeError) { PM::WeightedRoundRobin.new([1, { weight: 2 }]) }
    assert_equal 'weights must be an Integer', error.message
    error = assert_raises(RangeError) { PM::DeficitRoundRobin.new([0]) }
    assert_equal 'quanta must be in 1..=4294967295', error.message
    error = assert_raises(RangeError) { PM::DeficitRoundRobin.new([1]).select([1, 1]) }
    assert_equal 'heads must hold one entry per lane: 2 given, 1 lanes', error.message
    error = assert_raises(TypeError) { PM::DeficitRoundRobin.new([1]).deactivate(nil) }
    assert_equal 'lane must be an Integer', error.message
    error = assert_raises(RangeError) { PM::DeficitRoundRobin.new([1]).deactivate(-1) }
    assert_equal "lane must be in 0..=#{(2**64) - 1}", error.message
  end

  # Ready flags are exactly true or false, never truthiness, and the refusal
  # names the argument the same way for both schedulers that take them.
  def test_ready_flags_are_exactly_booleans
    weighted = PM::WeightedRoundRobin.new([1, 1])
    round_robin = PM::RoundRobin.new
    [nil, 0, 1, 'true', :true, Object.new].each do |flag|
      error = assert_raises(TypeError) { weighted.select([true, flag]) }
      assert_equal 'ready must be true or false', error.message
      error = assert_raises(TypeError) { round_robin.select([true, flag]) }
      assert_equal 'ready must be true or false', error.message
    end
    assert_equal [0, 1], [weighted.select([true, true]), weighted.select([true, true])]
    assert_equal 0, round_robin.cursor
  end

  # A deficit head is nil (an empty lane) or a positive Integer, under the
  # argument's name.
  def test_deficit_heads_are_nil_or_positive
    deficit = PM::DeficitRoundRobin.new([1, 1])
    error = assert_raises(RangeError) { deficit.select([nil, 0]) }
    assert_equal 'heads must be in 1..=4294967295', error.message
    error = assert_raises(TypeError) { deficit.select([nil, false]) }
    assert_equal 'heads must be an Integer', error.message
    assert_equal 1, deficit.select([nil, 4_294_967_295])
  end

  def test_only_the_core_operations_are_exposed
    assert_equal %i[cursor select], PM::RoundRobin.public_instance_methods(false).sort
    [PM::WeightedRoundRobin, PM::DeficitRoundRobin].each do |scheduler|
      assert_equal %i[deactivate select], scheduler.public_instance_methods(false).sort
    end
  end

  def test_freeze_gc_and_instances
    [PM::WeightedRoundRobin.new([1]), PM::DeficitRoundRobin.new([1])].each do |scheduler|
      scheduler.freeze
      assert_raises(FrozenError) { scheduler.select([true]) }
      assert_raises(FrozenError) { scheduler.deactivate(0) }
      assert_raises(FrozenError) { scheduler.select(nil) }
      assert_raises(FrozenError) { scheduler.select([Object.new]) }
      assert_raises(FrozenError) { scheduler.deactivate(nil) }
      assert_raises(FrozenError) { scheduler.deactivate(-1) }
    end
    # No instance exists without the core behind it.
    [PM::RoundRobin, PM::WeightedRoundRobin, PM::DeficitRoundRobin].each do |scheduler|
      assert_raises(TypeError) { scheduler.allocate }
    end
    first = PM::WeightedRoundRobin.new([2, 1])
    first.select([true, true])
    GC.start
    GC.compact
    second = PM::WeightedRoundRobin.new([2, 1])
    assert_equal 0, second.select([true, true])
    assert_equal 0, first.select([true, true])
    assert_equal 1, first.select([true, true])
    assert_raises(TypeError) { first.deactivate(0.1) }
    assert_raises(RangeError) { first.deactivate(-1) }
    assert_raises(RangeError) { first.deactivate(2**64) }
  end

  # Textbook burst WRR over three lanes, kept apart from the native state.
  class Model
    def initialize(weights)
      @weights = weights
      @lane = 0
      @left = weights.first
    end

    def state = [@lane, @left]

    def pass
      @lane = (@lane + 1) % @weights.size
      @left = @weights[@lane]
    end

    def select(ready)
      @weights.size.times do
        lane = @lane
        unless ready[lane]
          pass
          next
        end
        @left -= 1
        pass if @left.zero?
        return lane
      end
      nil
    end

    def deactivate(lane)
      return :unknown_lane if lane >= @weights.size

      return :idle unless lane == @lane && @left < @weights[lane]

      pass
      :deactivated
    end
  end

  def test_randomized_trace_matches_a_reference_model
    random = Random.new(4215)
    scheduler = PM::WeightedRoundRobin.new([3, 1, 2])
    model = Model.new([3, 1, 2])
    seen = Hash.new(0)

    6_000.times do
      if random.rand(4).zero?
        lane = random.rand(5)
        outcome = scheduler.deactivate(lane)
        assert_equal model.deactivate(lane), outcome
        seen[outcome] += 1
      else
        ready = Array.new(3) { random.rand(2).zero? }
        assert_equal [model.select(ready)], [scheduler.select(ready)]
      end
    end
    assert_operator seen.values_at(:deactivated, :idle, :unknown_lane).min, :>, 50
  end
end

# frozen_string_literal: true

require 'minitest/autorun'
require 'priority_machines'

class PriorityMachinesTest < Minitest::Test
  def test_weighted_reference_and_exact_lane_counts
    (1..16).each do |n|
      scheduler = PriorityMachines::WeightedRoundRobin.new(weights: (1..n).to_a)
      assert_equal n, scheduler.lane_count
      expected = (0...n).flat_map { |i| [i] * (i + 1) }
      assert_equal expected, expected.map { scheduler.select([true] * n) }
      assert_nil scheduler.select([false] * n)
    end
  end

  def test_deficit_reference
    scheduler = PriorityMachines::DeficitRoundRobin.new(quanta: [3, 1])
    assert_equal [0, 0, 0, 1] * 5, Array.new(20) { scheduler.select([1, 1]) }
    assert_nil scheduler.select([nil, nil])
    assert_equal 0, scheduler.select([4_294_967_295, nil])
    assert scheduler.deactivate(0)
    refute scheduler.deactivate(99)
  end

  def test_earliest_deadline
    assert_nil PriorityMachines.earliest_deadline([])
    assert_nil PriorityMachines.earliest_deadline([nil, nil])
    assert_equal 1, PriorityMachines.earliest_deadline([nil, 1, 1.0])
    assert_equal 2, PriorityMachines.earliest_deadline([10, nil, 0.25])
    assert_equal 1, PriorityMachines.earliest_deadline([2**63 + 1, 2**63])
  end

  def test_entire_call_validated_before_mutation
    weighted = PriorityMachines::WeightedRoundRobin.new(weights: [2, 1])
    assert_raises(TypeError) { weighted.select([true, Object.new]) }
    assert_raises(ArgumentError) { weighted.select([true]) }
    assert_equal [0, 0, 1], Array.new(3) { weighted.select([true, true]) }
    deficit = PriorityMachines::DeficitRoundRobin.new(quanta: [2, 1])
    assert_raises(TypeError) { deficit.select([1, 1.5]) }
    assert_raises(ArgumentError) { deficit.select([1, 0]) }
    assert_equal [0, 0, 1], Array.new(3) { deficit.select([1, 1]) }
  end

  def test_numeric_boundaries
    [[], [1] * 17, [0]].each do |weights|
      assert_raises(ArgumentError) { PriorityMachines::WeightedRoundRobin.new(weights:) }
    end
    [1.5, '1', Object.new].each do |value|
      assert_raises(TypeError) { PriorityMachines::DeficitRoundRobin.new(quanta: [value]) }
    end
    [-1, 2**32].each do |value|
      assert_raises(RangeError) { PriorityMachines::WeightedRoundRobin.new(weights: [value]) }
    end
    [Float::NAN, Float::INFINITY, -0.1, 1e100].each do |value|
      assert_raises(ArgumentError) { PriorityMachines.earliest_deadline([value]) }
    end
    [-1, 2**64].each do |value|
      assert_raises(RangeError) { PriorityMachines.earliest_deadline([value]) }
    end
    assert_raises(TypeError) { PriorityMachines.earliest_deadline(['1']) }
  end

  def test_freeze_gc_and_instances
    [PriorityMachines::WeightedRoundRobin.new(weights: [1]),
     PriorityMachines::DeficitRoundRobin.new(quanta: [1])].each do |scheduler|
      scheduler.freeze
      assert_raises(FrozenError) { scheduler.select([true]) }
      assert_raises(FrozenError) { scheduler.deactivate(0) }
      assert_equal 1, scheduler.lane_count
    end
    first = PriorityMachines::WeightedRoundRobin.new(weights: [2, 1])
    first.select([true, true])
    GC.start
    GC.compact
    second = PriorityMachines::WeightedRoundRobin.new(weights: [2, 1])
    assert_equal 0, second.select([true, true])
    assert_equal 0, first.select([true, true])
    assert_equal 1, first.select([true, true])
    assert_raises(TypeError) { first.deactivate(0.1) }
  end
end

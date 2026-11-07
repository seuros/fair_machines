# frozen_string_literal: true

require 'minitest/autorun'
require 'fair_machines'

class KeyedFairTest < Minitest::Test
  def test_lanes_keep_their_own_budget
    fair = FairMachines::KeyedFair.new(lanes: 4, capacity: 2)
    hot, cold = 0, -1
    refute_equal fair.lane(hot), fair.lane(cold)
    lane = fair.admit(hash: hot, cost: 2)
    assert_equal fair.lane(hot), lane
    assert_nil fair.admit(hash: hot, cost: 1)
    assert fair.admit(hash: cold, cost: 2)
    assert_equal 2, fair.in_flight(lane)
    assert fair.release(lane:, cost: 2)
    refute fair.release(lane:, cost: 1)
    refute fair.release(lane: 4, cost: 1)
    assert_equal 0, fair.in_flight(4)
  end

  def test_any_ruby_hash_is_accepted
    fair = FairMachines::KeyedFair.new(lanes: 256, capacity: 1)
    [:org.hash, 'org'.hash, 2**64 - 1, -(2**63)].each do |hash|
      assert_operator fair.lane(hash), :<, 256
    end
    assert_raises(TypeError) { fair.lane(1.5) }
    assert_raises(RangeError) { fair.lane(2**64) }
  end

  def test_strict_construction
    assert_equal 64, FairMachines::KeyedFair.new(lanes: 64, capacity: 1).lane_count
    [0, 3, 512].each do |lanes|
      assert_raises(ArgumentError) { FairMachines::KeyedFair.new(lanes:, capacity: 1) }
    end
    assert_raises(ArgumentError) { FairMachines::KeyedFair.new(lanes: 4, capacity: 0) }
    assert_raises(ArgumentError) { FairMachines::KeyedFair.new(lanes: 4, capacity: 1).admit(hash: 0, cost: 0) }
  end

  def test_frozen
    fair = FairMachines::KeyedFair.new(lanes: 2, capacity: 1).freeze
    assert_raises(FrozenError) { fair.admit(hash: 0, cost: 1) }
    assert_raises(FrozenError) { fair.release(lane: 0, cost: 1) }
  end
end

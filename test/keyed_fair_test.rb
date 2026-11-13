# frozen_string_literal: true

require 'minitest/autorun'
require 'fair_machines'

class KeyedFairTest < Minitest::Test
  KeyedFair = FairMachines::KeyedFair
  Lane = FairMachines::Lane
  LAST = (2**64) - 1

  def test_lanes_keep_their_own_budget
    fair = KeyedFair.new(4, 2)
    hot = 0
    lane = fair.admit(hot, 2)
    assert_equal Lane.new(0), lane
    assert_equal KeyedFair.lane(4, hot), lane
    refute_equal lane, KeyedFair.lane(4, LAST)
    assert_equal :insufficient, fair.admit(hot, 1)
    assert_equal Lane.new(3), fair.admit(LAST, 2)
    assert_equal [2, 0], [fair.in_flight(lane), fair.available(lane)]
    assert_equal :released, fair.release(lane, 2)
    assert_equal [0, 2], [fair.in_flight(lane), fair.available(lane)]
  end

  def test_refusals_are_symbols_and_change_nothing
    fair = KeyedFair.new(4, 2)
    lane = fair.admit(0, 1)
    before = Array.new(4) { |index| fair.in_flight(Lane.new(index)) }
    assert_equal :insufficient, fair.admit(0, 2)
    assert_equal :exceeds_in_flight, fair.release(lane, 2)
    assert_equal :unknown_lane, fair.release(Lane.new(4), 1)
    assert_equal :unknown_lane, fair.release(Lane.new(LAST), 1)
    assert_equal before, Array.new(4) { |index| fair.in_flight(Lane.new(index)) }
    assert_nil fair.in_flight(Lane.new(4))
    assert_nil fair.available(Lane.new(4))
    # Lanes count cost, not work: a repeat is honoured only while cost remains.
    assert_equal :released, fair.release(lane, 1)
    assert_equal :exceeds_in_flight, fair.release(lane, 1)
  end

  def test_a_lane_is_a_bucket_position
    assert_kind_of Data, KeyedFair.lane(4, 0)
    assert_equal [:index], Lane.members
    assert_equal [0, 1, 2, 3], [0, 2**62, 2**63, LAST].map { |hash| KeyedFair.lane(4, hash).index }
    assert_equal [0, 0, 1, 2], [0, LAST / 3, LAST / 3 + 1, LAST].map { |hash| KeyedFair.lane(3, hash).index }
    assert_equal Lane.new(1), KeyedFair.lane_at(2, 1)
    assert_nil KeyedFair.lane_at(2, 2)
    assert_nil KeyedFair.lane_at(2, LAST)
    # The same hash is the same lane in every instance with as many lanes.
    assert_equal KeyedFair.new(8, 1).admit(LAST, 1), KeyedFair.new(8, 5).admit(LAST, 1)
  end

  def test_every_lane_count_up_to_the_binding_limit
    assert_equal 256, KeyedFair::MAX_LANES
    (1..KeyedFair::MAX_LANES).each do |lanes|
      fair = KeyedFair.new(lanes, 1)
      assert_equal lanes, fair.lane_count
      assert_equal lanes - 1, fair.admit(LAST, 1).index
      assert_equal lanes - 1, KeyedFair.lane(lanes, LAST).index
      assert_nil KeyedFair.lane_at(lanes, lanes)
    end
  end

  def test_hashes_are_unsigned
    fair = KeyedFair.new(256, 1)
    [:org.hash, 'org'.hash].each do |hash|
      assert_operator fair.admit(hash & LAST, 1).index, :<, 256
    end
    [-1, 2**64].each do |hash|
      assert_raises(RangeError) { fair.admit(hash, 1) }
      assert_raises(RangeError) { KeyedFair.lane(256, hash) }
    end
    assert_raises(TypeError) { KeyedFair.lane(256, 1.5) }
    assert_raises(TypeError) { fair.admit('0', 1) }
  end

  def test_strict_construction_and_arguments
    [0, 257, 512].each do |lanes|
      assert_raises(ArgumentError) { KeyedFair.new(lanes, 1) }
      assert_raises(ArgumentError) { KeyedFair.lane(lanes, 0) }
      assert_raises(ArgumentError) { KeyedFair.lane_at(lanes, 0) }
    end
    assert_raises(RangeError) { KeyedFair.new(-1, 1) }
    assert_raises(RangeError) { KeyedFair.new(4, 0) }
    assert_raises(TypeError) { KeyedFair.new(4.0, 1) }
    assert_raises(ArgumentError) { KeyedFair.new(lanes: 4, capacity: 1) }
    assert_raises(TypeError) { KeyedFair.allocate }
    fair = KeyedFair.new(4, 1)
    assert_raises(RangeError) { fair.admit(0, 0) }
    assert_raises(TypeError) { fair.release(0, 1) }
    assert_raises(TypeError) { fair.in_flight(0) }
    assert_raises(TypeError) { fair.release(Lane.new('0'), 1) }
    assert_raises(RangeError) { fair.release(Lane.new(-1), 1) }
    assert_raises(RangeError) { fair.release(Lane.new(0), 0) }
  end

  def test_frozen
    fair = KeyedFair.new(2, 1).freeze
    assert_raises(FrozenError) { fair.admit(0, 1) }
    assert_raises(FrozenError) { fair.release(Lane.new(0), 1) }
    assert_equal [2, 0, 1], [fair.lane_count, fair.in_flight(Lane.new(0)), fair.available(Lane.new(0))]
  end
end

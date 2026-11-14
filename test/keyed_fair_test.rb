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
      assert_raises(RangeError) { KeyedFair.new(lanes, 1) }
      assert_raises(RangeError) { KeyedFair.lane(lanes, 0) }
      assert_raises(RangeError) { KeyedFair.lane_at(lanes, 0) }
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
    assert_raises(RangeError) { fair.release(Lane.new(0), 0) }
  end

  def test_messages_name_the_argument
    fair = KeyedFair.new(4, 2)
    {
      -> { KeyedFair.new(0, 1) } => [RangeError, 'lanes must be in 1..=256'],
      -> { KeyedFair.new(4, nil) } => [TypeError, 'capacity must be an Integer'],
      -> { KeyedFair.lane(4, -1) } => [RangeError, "hash must be in 0..=#{LAST}"],
      -> { KeyedFair.lane_at(4, 1.5) } => [TypeError, 'index must be an Integer'],
      -> { fair.admit(0, 0) } => [RangeError, 'cost must be in 1..=4294967295'],
      -> { fair.release(0, 1) } => [TypeError, 'lane must be a FairMachines::Lane'],
      -> { Lane.new(-1) } => [RangeError, "index must be in 0..=#{LAST}"]
    }.each do |call, (error, message)|
      assert_equal message, assert_raises(error, &call).message
    end
  end

  def test_a_lane_is_valid_or_does_not_exist
    ['0', 1.5, nil, :a].each { |index| assert_raises(TypeError) { Lane.new(index) } }
    [-1, 2**64].each { |index| assert_raises(RangeError) { Lane.new(index) } }
    assert_raises(RangeError) { Lane.new(index: -1) }
    assert_raises(RangeError) { Lane.new(3).with(index: -1) }
    assert_raises(ArgumentError) { Lane.new }
    assert_equal Lane.new(3), Lane.new(index: 3)
    assert_equal LAST, Lane.new(LAST).index
    refute_respond_to FairMachines, :__lane
  end

  # Runs `hook` inside `Lane.new` while it is set, without redefining `new`.
  module LaneHook
    attr_accessor :hook

    def new(...)
      hook&.call
      super
    end
  end
  Lane.singleton_class.prepend(LaneHook)

  def with_lane_hook(hook)
    Lane.hook = hook
    yield
  ensure
    Lane.hook = nil
  end

  def budgets(fair)
    Array.new(fair.lane_count) { |index| [fair.in_flight(Lane.new(index)), fair.available(Lane.new(index))] }
  end

  # The binding never loses work: the core admits on a copy, kept only once
  # the Lane is built and the receiver is still unfrozen and unchanged.
  def test_admit_charges_nothing_when_the_lane_cannot_be_built
    fair = KeyedFair.new(4, 2)
    before = budgets(fair)
    with_lane_hook(-> { raise 'no lane' }) do
      assert_equal 'no lane', assert_raises(RuntimeError) { fair.admit(0, 1) }.message
    end
    assert_equal before, budgets(fair)
  end

  def test_admit_reentered_while_the_lane_is_built_applies_nothing
    fair = KeyedFair.new(4, 2)
    with_lane_hook(-> { Lane.hook = nil; fair.admit(LAST, 1) }) do
      error = assert_raises(RuntimeError) { fair.admit(0, 1) }
      assert_match(/changed while the answer was built/, error.message)
    end
    # Only the inner admission (lane 3) happened; the outer one (lane 0) did not.
    assert_equal [[0, 2], [0, 2], [0, 2], [1, 1]], budgets(fair)
  end

  def test_admit_frozen_while_the_lane_is_built_applies_nothing
    fair = KeyedFair.new(4, 2)
    with_lane_hook(-> { fair.freeze }) do
      assert_raises(FrozenError) { fair.admit(0, 1) }
    end
    assert_equal [[0, 2]] * 4, budgets(fair)
  end

  def test_frozen
    fair = KeyedFair.new(2, 1).freeze
    assert_raises(FrozenError) { fair.admit(0, 1) }
    assert_raises(FrozenError) { fair.release(Lane.new(0), 1) }
    # Frozen is checked first, before the arguments are read.
    assert_raises(FrozenError) { fair.admit(-1, 0) }
    assert_raises(FrozenError) { fair.release(0, nil) }
    assert_equal [2, 0, 1], [fair.lane_count, fair.in_flight(Lane.new(0)), fair.available(Lane.new(0))]
  end
end

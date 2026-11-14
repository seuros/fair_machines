# frozen_string_literal: true

require 'minitest/autorun'
require 'fair_machines'

# Lane is sealed, and sealing stops accidents, not reflection: the binding
# checks the exact class of every Lane it reads and re-validates its index.
class SealedTest < Minitest::Test
  Lane = FairMachines::Lane
  KeyedFair = FairMachines::KeyedFair

  # A named subclass raises, but Ruby binds the constant before `inherited`
  # runs, so `ForgedLane` leaks; reopened, it skips Lane's validation.
  begin
    class ForgedLane < FairMachines::Lane; end
  rescue TypeError => e
    NAMED_SUBCLASS_ERROR = e
  end

  class ForgedLane
    def initialize(index:) = Data.instance_method(:initialize).bind_call(self, index:)
  end

  def test_lane_is_sealed
    assert_raises(NoMethodError) { Lane.allocate }
    assert_raises(TypeError) { Class.new(Lane) }
    assert_kind_of TypeError, NAMED_SUBCLASS_ERROR
    assert_equal Lane, ForgedLane.superclass, 'the constant leaks'
  end

  def test_a_forged_lane_never_reaches_the_core
    fair = KeyedFair.new(4, 2)
    lane = fair.admit(0, 1)
    [ForgedLane.new(0), ForgedLane.new(-5)].each do |forged|
      assert_raises(TypeError) { fair.release(forged, 1) }
      assert_raises(TypeError) { fair.in_flight(forged) }
      assert_raises(TypeError) { fair.available(forged) }
    end
    assert_equal [1, 1], [fair.in_flight(lane), fair.available(lane)]
  end

  def test_a_marshalled_lane_is_revalidated
    forged = Marshal.load(Marshal.dump(Lane.new(1)).gsub("i\x06".b, "i\xFA".b))
    assert_instance_of Lane, forged
    assert_equal(-1, forged.index)
    fair = KeyedFair.new(4, 2)
    assert_raises(RangeError) { fair.release(forged, 1) }
    assert_raises(RangeError) { fair.in_flight(forged) }
    assert_equal Lane.new(1), Marshal.load(Marshal.dump(Lane.new(1)))
  end

  def test_a_lane_with_a_singleton_class_is_still_a_lane
    lane = Lane.new(2)
    lane.singleton_class
    assert_equal 0, KeyedFair.new(4, 2).in_flight(lane)
  end
end

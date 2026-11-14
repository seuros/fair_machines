# frozen_string_literal: true

require 'minitest/autorun'
require 'fair_machines'

class FairMachinesTest < Minitest::Test
  TenantFair = FairMachines::TenantFair

  def scheduler = TenantFair.new([20, 20], [3, 1])

  def snapshot(fair) = Array.new(fair.tenant_count) { |tenant| [fair.in_flight(tenant), fair.available(tenant)] }

  def test_budgets_and_fair_cost_reference
    fair = scheduler
    assert_equal :admitted, fair.admit(0, 20)
    assert_equal :admitted, fair.admit(1, 20)
    assert_equal :insufficient, fair.admit(0, 1)
    assert_equal [0, 0, 0, 1] * 5, Array.new(20) { fair.select([1, 1]) }
    assert_equal [[5, 15], [15, 5]], snapshot(fair)
    assert_equal :admitted, fair.admit(0, 15)
    assert_equal :insufficient, fair.admit(0, 1)
    assert_equal :cancelled, fair.cancel(1, 15)
    assert_equal :exceeds_in_flight, fair.cancel(1, 1)
  end

  def test_exact_tenant_counts
    assert_equal 16, TenantFair::MAX_TENANTS
    (1..TenantFair::MAX_TENANTS).each do |n|
      fair = TenantFair.new([1] * n, [1] * n)
      assert_equal n, fair.tenant_count
      n.times { |tenant| assert_equal :admitted, fair.admit(tenant, 1) }
      assert_equal (0...n).to_a, Array.new(n) { |turn| fair.select([nil] * turn + [1] * (n - turn)) }
      assert_nil fair.select([nil] * n)
    end
  end

  def test_select_is_nil_only_when_every_head_is_nil
    fair = scheduler
    assert_nil fair.select([nil, nil])
    assert_equal :admitted, fair.admit(1, 1)
    assert_nil fair.select([nil, nil])
    assert_equal 1, fair.select([nil, 1])
    assert_nil fair.select([nil, nil])
  end

  def test_an_unadmitted_head_refuses_the_whole_selection
    fair = scheduler
    assert_equal :exceeds_in_flight, fair.select([1, 1])
    assert_equal :admitted, fair.admit(1, 1)
    before = snapshot(fair)
    assert_equal :exceeds_in_flight, fair.select([20, 1])
    assert_equal :exceeds_in_flight, fair.select([nil, 2])
    assert_equal before, snapshot(fair)
    assert_equal 1, fair.select([nil, 1])
  end

  def test_refusals_are_symbols_and_change_nothing
    fair = scheduler
    fair.admit(0, 3)
    fair.admit(1, 1)
    before = snapshot(fair)
    assert_equal :unknown_tenant, fair.admit(2, 1)
    assert_equal :unknown_tenant, fair.admit(2**64 - 1, 1)
    assert_equal :unknown_tenant, fair.cancel(2, 1)
    assert_equal :insufficient, fair.admit(0, 18)
    assert_equal :exceeds_in_flight, fair.cancel(1, 2)
    assert_equal :exceeds_in_flight, fair.select([4, 1])
    assert_equal before, snapshot(fair)
    assert_nil fair.in_flight(2)
    assert_nil fair.available(2)
    # The rotation was not disturbed either.
    assert_equal [0, 0, 0, 1], [[1, 1], [1, 1], [1, 1], [nil, 1]].map { |heads| fair.select(heads) }
  end

  def test_malformed_input_raises_before_anything_changes
    fair = scheduler
    fair.admit(0, 3)
    fair.admit(1, 1)
    assert_raises(TypeError) { fair.select([1, 1.5]) }
    assert_raises(RangeError) { fair.select([1, 0]) }
    assert_raises(RangeError) { fair.select([1]) }
    assert_raises(TypeError) { fair.select(1) }
    assert_raises(TypeError) { fair.cancel(0, 1.2) }
    assert_raises(TypeError) { fair.admit('0', 1) }
    assert_raises(ArgumentError) { fair.admit(0) }
    assert_raises(ArgumentError) { fair.admit(tenant: 0, cost: 1) }
    assert_equal [[3, 17], [1, 19]], snapshot(fair)
    assert_equal [0, 0, 0, 1], [[1, 1], [1, 1], [1, 1], [nil, 1]].map { |heads| fair.select(heads) }
  end

  def test_strict_boundary
    [[], [1] * 17].each do |capacities|
      assert_raises(RangeError) { TenantFair.new(capacities, [1] * capacities.size) }
    end
    assert_raises(RangeError) { TenantFair.new([1], [1, 1]) }
    assert_raises(ArgumentError) { TenantFair.new([1]) }
    assert_raises(TypeError) { TenantFair.new(1, [1]) }
    assert_raises(TypeError) { TenantFair.allocate }
    assert_raises(RangeError) { TenantFair.new([0], [1]) }
    assert_raises(RangeError) { TenantFair.new([1], [0]) }
    [1.5, '1', Object.new, nil].each do |v|
      assert_raises(TypeError) { TenantFair.new([v], [1]) }
    end
    [-1, 0, 2**32].each do |v|
      assert_raises(RangeError) { scheduler.admit(0, v) }
      assert_raises(RangeError) { scheduler.cancel(0, v) }
    end
    [-1, 2**64].each do |v|
      assert_raises(RangeError) { scheduler.admit(v, 1) }
      assert_raises(RangeError) { scheduler.in_flight(v) }
    end
  end

  def test_messages_name_the_argument
    {
      -> { TenantFair.new([1.5], [1]) } => [TypeError, 'capacities must be an Integer'],
      -> { TenantFair.new([1], [0]) } => [RangeError, 'quanta must be in 1..=4294967295'],
      -> { TenantFair.new([1], Class.new(Array).new([1])) } => [TypeError, 'quanta must be an Array'],
      -> { scheduler.select(Class.new(Array).new([1, 1])) } => [TypeError, 'heads must be an Array'],
      -> { TenantFair.new([1] * 17, [1] * 17) } => [RangeError, 'capacities must hold 1..=16 tenants'],
      -> { scheduler.admit('0', 1) } => [TypeError, 'tenant must be an Integer'],
      -> { scheduler.in_flight(2**64) } => [RangeError, "tenant must be in 0..=#{(2**64) - 1}"],
      -> { scheduler.cancel(0, 0) } => [RangeError, 'cost must be in 1..=4294967295'],
      -> { scheduler.select([1, 1.5]) } => [TypeError, 'heads must be an Integer'],
      -> { scheduler.select([nil, 0]) } => [RangeError, 'heads must be in 1..=4294967295'],
      -> { scheduler.select([1]) } => [RangeError, 'heads must hold one entry per tenant: 1 given, 2 tenants']
    }.each do |call, (error, message)|
      assert_equal message, assert_raises(error, &call).message
    end
  end

  def test_freeze_gc_and_independent_state
    fair = scheduler
    fair.admit(0, 1)
    GC.start
    GC.compact
    assert_equal 0, fair.select([1, nil])
    assert_equal :exceeds_in_flight, scheduler.select([1, nil])
    fair.freeze
    assert_raises(FrozenError) { fair.admit(0, 1) }
    assert_raises(FrozenError) { fair.cancel(0, 1) }
    assert_raises(FrozenError) { fair.select([nil, nil]) }
    # Frozen is checked first, before the arguments are read.
    assert_raises(FrozenError) { fair.admit('0', 1.5) }
    assert_raises(FrozenError) { fair.select(:nope) }
    assert_equal 2, fair.tenant_count
    assert_equal [[0, 20], [0, 20]], snapshot(fair)
  end
end

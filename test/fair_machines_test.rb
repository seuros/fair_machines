# frozen_string_literal: true

require 'minitest/autorun'
require 'fair_machines'

class FairMachinesTest < Minitest::Test
  def scheduler
    FairMachines::TenantFair.new(capacities: [20, 20], quanta: [3, 1])
  end

  def test_budgets_and_fair_cost_reference
    fair = scheduler
    assert fair.admit(tenant: 0, cost: 20)
    assert fair.admit(tenant: 1, cost: 20)
    refute fair.admit(tenant: 0, cost: 1)
    assert_equal [0, 0, 0, 1] * 5, Array.new(20) { fair.select([1, 1]) }
    assert fair.admit(tenant: 0, cost: 15)
    refute fair.admit(tenant: 0, cost: 1)
    assert fair.cancel(tenant: 1, cost: 15)
    refute fair.cancel(tenant: 1, cost: 1)
  end

  def test_exact_tenant_counts
    (1..16).each do |n|
      fair = FairMachines::TenantFair.new(capacities: [1] * n, quanta: [1] * n)
      assert_equal n, fair.tenant_count
      n.times { |tenant| assert fair.admit(tenant:, cost: 1) }
      assert_equal (0...n).to_a, Array.new(n) { fair.select([1] * n) }
      assert_nil fair.select([1] * n)
    end
  end

  def test_invalid_or_unadmitted_heads_and_cancel
    fair = scheduler
    assert_nil fair.select([1, 1])
    assert fair.admit(tenant: 1, cost: 1)
    assert_equal 1, fair.select([20, 1])
    refute fair.admit(tenant: 2, cost: 1)
    refute fair.cancel(tenant: 2, cost: 1)
    assert_nil fair.select([nil, nil])
  end

  def test_atomic_validation
    fair = scheduler
    fair.admit(tenant: 0, cost: 3)
    fair.admit(tenant: 1, cost: 1)
    assert_raises(TypeError) { fair.select([1, 1.5]) }
    assert_raises(ArgumentError) { fair.select([1, 0]) }
    assert_raises(ArgumentError) { fair.select([1]) }
    assert_raises(TypeError) { fair.cancel(tenant: 0, cost: 1.2) }
    assert_equal [0, 0, 0, 1], Array.new(4) { fair.select([1, 1]) }
  end

  def test_strict_boundary
    [[], [1] * 17, [0]].each do |capacities|
      assert_raises(ArgumentError) { FairMachines::TenantFair.new(capacities:, quanta: [1] * capacities.size) }
    end
    assert_raises(ArgumentError) { FairMachines::TenantFair.new(capacities: [1], quanta: [1, 1]) }
    [1.5, '1', Object.new].each do |v|
      assert_raises(TypeError) { FairMachines::TenantFair.new(capacities: [v], quanta: [1]) }
    end
    [-1, 2**32].each do |v|
      assert_raises(RangeError) { scheduler.admit(tenant: 0, cost: v) }
      assert_raises(RangeError) { scheduler.cancel(tenant: v, cost: 1) }
    end
    assert_raises(ArgumentError) { scheduler.admit(tenant: 0, cost: 0) }
    assert_raises(ArgumentError) { scheduler.admit(tenant: 0, cost: 1, extra: 2) }
  end

  def test_freeze_gc_and_independent_state
    fair = scheduler
    fair.admit(tenant: 0, cost: 1)
    GC.start
    GC.compact
    assert_equal 0, fair.select([1, nil])
    assert_nil scheduler.select([1, nil])
    fair.freeze
    assert_raises(FrozenError) { fair.admit(tenant: 0, cost: 1) }
    assert_raises(FrozenError) { fair.cancel(tenant: 0, cost: 1) }
    assert_raises(FrozenError) { fair.select([nil, nil]) }
    assert_equal 2, fair.tenant_count
  end
end

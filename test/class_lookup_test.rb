# frozen_string_literal: true

require 'minitest/autorun'
require 'open3'
require 'rbconfig'

# The classes of the values the extension wraps (TenantFair, KeyedFair) are
# fixed when it loads. Nothing a caller does later, redefining a constant or
# `Object.const_get`, reaches a scheduler being built. Each case runs in a
# fresh process: the class is fixed once per process, at load.
class ClassLookupTest < Minitest::Test
  LIB = File.expand_path('../lib', __dir__)

  PRELUDE = <<~RUBY
    require 'fair_machines'
    F = FairMachines
    TenantFair = F::TenantFair
    KeyedFair = F::KeyedFair
  RUBY

  # Runs `body` after PRELUDE in a new Ruby; answers its last line of output.
  def isolated(body)
    out, err, status = Open3.capture3(RbConfig.ruby, '-I', LIB, '-e', PRELUDE + body)
    assert status.success?, "exited #{status.exitstatus}: #{err}"
    out.lines.last&.chomp
  end

  # Bug: a TenantFair constant redefined before the first scheduler made it
  # an instance of the new class, which has none of the scheduler's methods.
  def test_a_redefined_tenant_fair_constant_does_not_change_the_class
    assert_equal 'true admitted 0', isolated(<<~RUBY)
      F.send(:remove_const, :TenantFair)
      F.const_set(:TenantFair, Class.new)
      fair = TenantFair.new([2, 2], [1, 1])
      puts [fair.instance_of?(TenantFair), fair.admit(0, 2), fair.select([1, nil])].join(' ')
    RUBY
  end

  # Bug: the same for a KeyedFair.
  def test_a_redefined_keyed_fair_constant_does_not_change_the_class
    assert_equal 'true 0 released', isolated(<<~RUBY)
      F.send(:remove_const, :KeyedFair)
      F.const_set(:KeyedFair, Class.new)
      fair = KeyedFair.new(4, 2)
      lane = fair.admit(0, 2)
      puts [fair.instance_of?(KeyedFair), lane.index, fair.release(lane, 2)].join(' ')
    RUBY
  end

  # Bug: building the first scheduler ran `Object.const_get`, Ruby code in
  # the middle of a native call. No Ruby runs while one is built now.
  def test_building_a_scheduler_runs_no_const_get
    assert_equal '0 TenantFair KeyedFair', isolated(<<~RUBY)
      hits = 0
      Object.define_singleton_method(:const_get) do |*args|
        hits += 1
        super(*args)
      end
      tenants = TenantFair.new([2], [1])
      keyed = KeyedFair.new(4, 2)
      puts [hits, tenants.class.name.split('::').last, keyed.class.name.split('::').last].join(' ')
    RUBY
  end

  # Bug: a redefined `Object.const_get` that raised hit an `unwrap` in the
  # extension: an uncatchable fatal, and the process exited.
  def test_a_raising_const_get_does_not_kill_the_process
    assert_equal 'admitted 0', isolated(<<~RUBY)
      Object.define_singleton_method(:const_get) { |*| raise IOError, 'lookup ran in a native call' }
      tenants = TenantFair.new([2], [1])
      keyed = KeyedFair.new(4, 2)
      puts [tenants.admit(0, 1), keyed.admit(0, 1).index].join(' ')
    RUBY
  end
end

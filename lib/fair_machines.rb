# frozen_string_literal: true

require_relative 'fair_machines/version'

module FairMachines
  # The core's `Lane`: the bucket a hash maps to, at position `index`.
  # A bucket position, not an issued id: `KeyedFair.lane`, `KeyedFair.lane_at`
  # and `Lane.new` build one without admitting anything, and holding one
  # proves nothing. Like `Lane::new`, it holds only a position: an Integer a
  # `usize` holds, TypeError for anything else, RangeError outside the range.
  Lane = Data.define(:index) do
    def initialize(index:)
      super(index: FairMachines.__send__(:__lane, index))
    end
  end
end

# Native is required: there is no pure-Ruby fallback. It validates and builds
# Lane values, so the Data class above must exist first.
require 'fair_machines_native/fair_machines_native'

module FairMachines
  private_class_method :__lane

  # Lane is final. `initialize` is where it validates, and a subclass could
  # override it, or `allocate` build an instance that never ran it:
  # subclassing raises TypeError and `allocate` is gone. This stops accidents,
  # not reflection, so the binding also checks every Lane it reads.
  Lane.singleton_class.undef_method(:allocate)
  Lane.define_singleton_method(:inherited) { |_| raise TypeError, "#{Lane} cannot be subclassed" }
end

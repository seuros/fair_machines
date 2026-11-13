# frozen_string_literal: true

require_relative 'fair_machines/version'

module FairMachines
  # The core's `Lane`: the bucket a hash maps to, `index` in 0...lane_count.
  # A bucket position, not an issued id: `KeyedFair.lane` and
  # `KeyedFair.lane_at` build one without admitting anything, and holding one
  # proves nothing.
  Lane = Data.define(:index)
end

# Native is required: there is no pure-Ruby fallback. It builds Lane values,
# so the Data class above must exist first.
require 'fair_machines_native/fair_machines_native'

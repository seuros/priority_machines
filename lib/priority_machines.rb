# frozen_string_literal: true

require_relative 'priority_machines/version'
require 'priority_machines_native/priority_machines_native'

module PriorityMachines
  # The schedulers wrap native state and have no allocator: a subclass's
  # `new` would build the base class anyway, so subclassing raises TypeError.
  [RoundRobin, WeightedRoundRobin, DeficitRoundRobin].each do |scheduler|
    scheduler.define_singleton_method(:inherited) { |_| raise TypeError, "#{scheduler} cannot be subclassed" }
  end
end

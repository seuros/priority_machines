# frozen_string_literal: true

require_relative 'lib/priority_machines/version'

Gem::Specification.new do |spec|
  spec.name = 'priority_machines'
  spec.version = PriorityMachines::VERSION
  spec.authors = ['Abdelkader Boudih']
  spec.email = ['oss@seuros.com']
  spec.summary = 'Native deadline, weighted and deficit scheduling policies'
  spec.description = 'Thin Ruby binding to the standalone allocation-free priority-machines Rust core. ' \
                     'Initial, unstable local implementation; caller owns queues and execution.'
  spec.homepage = 'https://github.com/seuros/priority_machines'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 4.0.0'
  spec.metadata['source_code_uri'] = spec.homepage
  spec.metadata['bug_tracker_uri'] = "#{spec.homepage}/issues"
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.metadata['cargo_crate_name'] = 'priority_machines_native'
  spec.metadata['cargo_manifest_path'] = 'ext/priority_machines_native/ffi/Cargo.toml'
  # README is required by the core's include_str! at build time.
  spec.files = Dir.glob(%w[lib/**/*.rb sig/**/*.rbs ext/**/*.{rb,rs,toml,md}
                          ext/**/LICENSE Cargo.toml Cargo.lock README.md CHANGELOG.md LICENSE.txt])
                  .select { |path| File.file?(path) }
                  .reject { |path| path.end_with?('/tests.rs') || path.include?('/tests/') }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/priority_machines_native/extconf.rb']
  spec.add_dependency 'rb_sys', '~> 0.9', '>= 0.9.130'
end

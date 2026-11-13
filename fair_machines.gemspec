# frozen_string_literal: true

require_relative 'lib/fair_machines/version'

Gem::Specification.new do |spec|
  spec.name = 'fair_machines'
  spec.version = FairMachines::VERSION
  spec.authors = ['Abdelkader Boudih']
  spec.email = ['oss@seuros.com']
  spec.summary = 'Native per-tenant admission budgets and deficit fairness'
  spec.description = 'Ruby adapter to the no_std fair-machines Rust core: per-tenant cost budgets with ' \
                     'deficit round robin dispatch, and hashed lanes for an open tenant set.'
  spec.homepage = 'https://github.com/seuros/fair_machines'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 4.0.0'
  spec.metadata['source_code_uri'] = spec.homepage
  spec.metadata['bug_tracker_uri'] = "#{spec.homepage}/issues"
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.metadata['cargo_crate_name'] = 'fair_machines_native'
  spec.metadata['cargo_manifest_path'] = 'ext/fair_machines_native/ffi/Cargo.toml'
  spec.files = Dir.glob(%w[lib/**/*.rb sig/**/*.rbs ext/**/*.{rb,rs,toml,md}
                          ext/**/LICENSE Cargo.toml Cargo.lock README.md CHANGELOG.md LICENSE.txt])
                  .select { |path| File.file?(path) }
                  .reject { |path| path.end_with?('/tests.rs') || path.include?('/tests/') }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/fair_machines_native/extconf.rb']
  spec.add_dependency 'rb_sys', '~> 0.9', '>= 0.9.130'
end

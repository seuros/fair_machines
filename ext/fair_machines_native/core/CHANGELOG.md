# Changelog

## [0.4.0](https://github.com/seuros/fair_machines/compare/fair-machines-v0.3.0...fair-machines-v0.4.0) (2026-11-13)


### ⚠ BREAKING CHANGES

* converted to the fleet conventions. One exhaustive Refusal per crate, Result from every operation that can refuse, a refusal changes nothing, provable idempotent repeats are honoured, Ruby mirrors the core. No compatibility aliases. Requires flow-machines >= 0.3 and priority-machines >= 0.4.

### Features

* answer Result&lt;_, Refusal&gt;, refuse unadmitted heads in select, mirror the core positionally in Ruby ([5e73ef6](https://github.com/seuros/fair_machines/commit/5e73ef67f86a0326709e27eca46d8f97bccd8aba))

## [0.3.0](https://github.com/seuros/fair_machines/compare/fair-machines-v0.2.2...fair-machines-v0.3.0) (2026-11-07)


### Features

* add KeyedFair hashed-lane budgets ([33d8f62](https://github.com/seuros/fair_machines/commit/33d8f62d6c8f8e8ad8a9bc759dffbdfa8b7a2706))

## [0.2.2](https://github.com/seuros/fair_machines/compare/fair-machines-v0.2.1...fair-machines-v0.2.2) (2026-11-06)


### Bug Fixes

* crates.io keywords and categories for the core ([c9e496f](https://github.com/seuros/fair_machines/commit/c9e496fca380c51529e965ce4c8158f5210ed258))

## [0.2.1](https://github.com/seuros/fair_machines/compare/fair-machines-v0.2.0...fair-machines-v0.2.1) (2026-11-06)


### Bug Fixes

* keep test sources out of the packaged gem and crate ([58a408b](https://github.com/seuros/fair_machines/commit/58a408ba6727d04d8517a6b76c47b90e381203e3))

## [0.2.0](https://github.com/seuros/fair_machines/compare/fair-machines-v0.1.0...fair-machines-v0.2.0) (2026-11-06)


### Features

* initial extraction ([3c25da7](https://github.com/seuros/fair_machines/commit/3c25da7ef06236f290cef43e29ce5519cbf69aff))

# Changelog

## [0.6.2](https://github.com/seuros/priority_machines/compare/priority_machines/v0.6.1...priority_machines/v0.6.2) (2026-11-16)


### Bug Fixes

* order every sibling instant as a deadline ([774eee5](https://github.com/seuros/priority_machines/commit/774eee5cd29bc27963d7901f91fe006765b19d16))

## [0.6.1](https://github.com/seuros/priority_machines/compare/priority_machines/v0.6.0...priority_machines/v0.6.1) (2026-11-16)


### Bug Fixes

* fix wrapped classes at load so building a value runs no Ruby ([8bbe272](https://github.com/seuros/priority_machines/commit/8bbe27297d20e068d7c1ec4e97fc8cdec66e8ee6))

## [0.6.0](https://github.com/seuros/priority_machines/compare/priority_machines/v0.5.1...priority_machines/v0.6.0) (2026-11-16)


### ⚠ BREAKING CHANGES

* WRR selects typed Lane with its head; schedulers sealed

### Features

* WRR selects typed Lane with its head; schedulers sealed ([0b09114](https://github.com/seuros/priority_machines/commit/0b09114068e976e2f4e9b6005cb66e9d00b8f4df))

## [0.5.1](https://github.com/seuros/priority_machines/compare/priority_machines/v0.5.0...priority_machines/v0.5.1) (2026-11-14)


### Bug Fixes

* **deps:** require matryona-machines &gt;= 0.4, build the Array refusal message lazily ([d1595f7](https://github.com/seuros/priority_machines/commit/d1595f79f2e157760d0c8b40d55e3f98413f0974))

## [0.5.0](https://github.com/seuros/priority_machines/compare/priority_machines/v0.4.0...priority_machines/v0.5.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* read ready flags and deficit heads with matryona boolean/optional_positive

### Features

* read ready flags and deficit heads with matryona boolean/optional_positive ([09676a3](https://github.com/seuros/priority_machines/commit/09676a33e1ff7e946b24e26ac825cea7747dd3a1))

## [0.4.0](https://github.com/seuros/priority_machines/compare/priority_machines/v0.3.0...priority_machines/v0.4.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* adopt matryona-machines 0.2 (exact arrays, frozen-first)

### Features

* adopt matryona-machines 0.2 (exact arrays, frozen-first) ([cdc7404](https://github.com/seuros/priority_machines/commit/cdc740470841f423b0e2fdfbcb8f37838ab2ba2e))

## [0.3.0](https://github.com/seuros/priority_machines/compare/priority_machines/v0.2.0...priority_machines/v0.3.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* raise RangeError for a wrong-length select and name every argument

### Features

* raise RangeError for a wrong-length select and name every argument ([d99d027](https://github.com/seuros/priority_machines/commit/d99d0273074e3ddedc44e428408b9e523eb5cfa8))

## [0.2.0](https://github.com/seuros/priority_machines/compare/priority_machines/v0.1.5...priority_machines/v0.2.0) (2026-11-13)


### ⚠ BREAKING CHANGES

* converted to the fleet conventions. One exhaustive Refusal per crate, Result from every operation that can refuse, a refusal changes nothing, provable idempotent repeats are honoured, Ruby mirrors the core. No compatibility aliases. A lane that holds nothing is Idle for any lane count and the cursor does not move.

### Features

* deactivate answers Result&lt;Deactivation, Refusal&gt;, Idle exactly when nothing changes, Ruby RoundRobin and nanosecond deadlines ([0ef6aad](https://github.com/seuros/priority_machines/commit/0ef6aad0fadeb511b0de49e1ada2a2b964fca594))

## [0.1.5](https://github.com/seuros/priority_machines/compare/priority_machines/v0.1.4...priority_machines/v0.1.5) (2026-11-06)


### Bug Fixes

* crates.io keywords and categories for the core ([1d389af](https://github.com/seuros/priority_machines/commit/1d389af51eacb70fe595028c381c68cb9d556f51))

## [0.1.4](https://github.com/seuros/priority_machines/compare/priority_machines/v0.1.3...priority_machines/v0.1.4) (2026-11-06)


### Bug Fixes

* keep test sources out of the packaged gem and crate ([37e58c0](https://github.com/seuros/priority_machines/commit/37e58c0575a088807db99d38ea820d87dde956b8))

## [0.1.3](https://github.com/seuros/priority_machines/compare/priority_machines/v0.1.2...priority_machines/v0.1.3) (2026-11-06)


### Features

* plain round robin scheduler in the core ([315e567](https://github.com/seuros/priority_machines/commit/315e5677ab93d677c71847233abcaa2bad5a7f4e))

## [0.1.2](https://github.com/seuros/priority_machines/compare/priority_machines/v0.1.1...priority_machines/v0.1.2) (2026-11-06)


### Bug Fixes

* cross builds no longer compile for the host ([dcf3aac](https://github.com/seuros/priority_machines/commit/dcf3aaccd145677feb6f61ea2fbd3d7e7f3e120a))

## [0.1.1](https://github.com/seuros/priority_machines/compare/priority_machines-v0.1.0...priority_machines/v0.1.1) (2026-11-06)


### Features

* initial extraction ([ea040c3](https://github.com/seuros/priority_machines/commit/ea040c3e486d5b77a9eb4c68a8ac493399df8323))

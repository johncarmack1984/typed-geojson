# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/johncarmack1984/typed-geojson/compare/v0.2.0...v0.2.1) - 2026-07-22

### Other

- sharpen discoverability metadata for 'typed geojson' search ([#19](https://github.com/johncarmack1984/typed-geojson/pull/19))

## [0.2.0](https://github.com/johncarmack1984/typed-geojson/compare/v0.1.4...v0.2.0) - 2026-07-22

### Added

- Preserve RFC 7946 §6.1 foreign members on `Feature` / `FeatureCollection`: unknown top-level members now round-trip byte-faithfully through serde and across the `geojson` bridge instead of being dropped, via a new public `foreign_members` field. They are intentionally excluded from the `specta`/TypeScript export, so the `@types/geojson` assignability contract is unchanged. ([#18](https://github.com/johncarmack1984/typed-geojson/pull/18))
- `geo-types` feature: `From` / `TryFrom` between the geometry types and `geo_types`, matching the underlying `geojson` crate's conversion semantics, so a typed geometry flows into the georust `geo` algorithms layer. ([#18](https://github.com/johncarmack1984/typed-geojson/pull/18))

### Changed

- **Breaking:** `Feature` and `FeatureCollection` have a new public `foreign_members` field, so struct-literal construction must set it — use `Feature::new` for the empty default. ([#18](https://github.com/johncarmack1984/typed-geojson/pull/18))

## [0.1.4](https://github.com/johncarmack1984/typed-geojson/compare/v0.1.3...v0.1.4) - 2026-07-18

### Other

- gate assignability from @types/geojson 7946.0.8 through latest, plus a tsc canary ([#15](https://github.com/johncarmack1984/typed-geojson/pull/15))

## [0.1.3](https://github.com/johncarmack1984/typed-geojson/compare/v0.1.2...v0.1.3) - 2026-07-07

### Other

- install rustfmt and clippy components with the channel ([#9](https://github.com/johncarmack1984/typed-geojson/pull/9))
- Update dependencies ([#6](https://github.com/johncarmack1984/typed-geojson/pull/6))

## [0.1.2](https://github.com/johncarmack1984/typed-geojson/compare/v0.1.1...v0.1.2) - 2026-07-04

### Other

- Publish via crates.io Trusted Publishing; commit Cargo.lock ([#5](https://github.com/johncarmack1984/typed-geojson/pull/5))
- Fix stale crate description; de-bold README; drop rustdoc em-dashes; add seo-kit baseline ([#4](https://github.com/johncarmack1984/typed-geojson/pull/4))
- add rust-toolchain.toml (stable); bump checkout to v7, CI Node to lts ([#3](https://github.com/johncarmack1984/typed-geojson/pull/3))
- tidy README punctuation

# typed-geojson

[![crates.io](https://img.shields.io/crates/v/typed-geojson.svg)](https://crates.io/crates/typed-geojson)
[![docs.rs](https://img.shields.io/docsrs/typed-geojson)](https://docs.rs/typed-geojson)
[![CI](https://img.shields.io/github/actions/workflow/status/johncarmack1984/typed-geojson/ci.yml?branch=main&label=CI)](https://github.com/johncarmack1984/typed-geojson/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/typed-geojson.svg)](#license)

Strongly-typed [GeoJSON](https://datatracker.ietf.org/doc/html/rfc7946) for Rust.
`Feature<G, P>` / `FeatureCollection<G, P>`: generic over **G**eometry and
**P**roperties, layered over the georust [`geojson`](https://crates.io/crates/geojson)
crate. With the `specta` feature it exports to TypeScript that is mutually
assignable with [`@types/geojson`](https://www.npmjs.com/package/@types/geojson).

```rust
use serde::{Deserialize, Serialize};
use typed_geojson::{Feature, Geometry};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Station { id: u32, name: String, temp_c: f64 }

let raw = r#"{
    "type": "Feature",
    "geometry": { "type": "Point", "coordinates": [-96.8, 32.8] },
    "properties": { "id": 7, "name": "DFW", "temp_c": 31.5 }
}"#;

let feature: Feature<Geometry, Station> = serde_json::from_str(raw).unwrap();
assert_eq!(feature.properties.name, "DFW");
```

## What you get

- `Feature<G, P>` / `FeatureCollection<G, P>`: native parameter order; `G`
  defaults to the [`Geometry`] union, `P` to an untyped JSON object.
- Typed geometry (`Point`, `LineString`, `Polygon`, `MultiPoint`,
  `MultiLineString`, `MultiPolygon`, `GeometryCollection`, and the `Geometry`
  union), each matching its native `@types/geojson` shape.
- serde that round-trips to/from spec GeoJSON: validates `"type"`, **preserves
  foreign members** (RFC 7946 §6.1), keeps `id`/`bbox`, omits an absent `bbox`.
- `TryFrom` bridges to/from the untyped `geojson` crate.
- Optional `geo-types` feature: `From`/`TryFrom` between the geometry types and
  [`geo-types`](https://crates.io/crates/geo-types), so a typed geometry flows
  into the georust [`geo`](https://crates.io/crates/geo) algorithms layer.

## Nullability lives in `G`

Geometry is required but may be `null` (RFC 7946); like native, that nullability
is a choice of `G`:

```rust
type Located   = Feature<Geometry, Props>;          // geometry: Geometry  (non-null)
type Unlocated = Feature<Option<Geometry>, Props>;  // geometry: Geometry | null
type PointFeat = Feature<Point, Props>;             // geometry: Point
```

## Foreign members are preserved, not dropped

RFC 7946 §6.1 lets a GeoJSON object carry members the spec doesn't define — an
extension's `"title"`, a producer's vendor field, a legacy `"crs"`. The standard
`geojson` crate's typed path discards these; this crate keeps them on
`Feature` and `FeatureCollection`, so a parse → serialize round-trip is
byte-faithful and they also survive the `geojson` bridge.

```rust
use typed_geojson::{Feature, Geometry};

let raw = r#"{"type":"Feature","geometry":null,"properties":null,"title":"Sensor 7"}"#;
let f: Feature<Option<Geometry>> = serde_json::from_str(raw).unwrap();
assert_eq!(f.foreign_members["title"], serde_json::json!("Sensor 7"));
assert_eq!(serde_json::to_string(&f).unwrap(), raw); // round-trips byte-for-byte
```

Foreign members are runtime fidelity only — they are deliberately left out of
the `specta`/TypeScript export, because `@types/geojson` doesn't type them
either, and keeping them out of the static contract is what preserves the
assignability guarantee.

## `geo-types` (feature)

Enable the `geo-types` feature to convert between the geometry types and
`geo-types`, matching upstream `geojson`'s semantics: `geo_types` → ours is
infallible (`From`; a 3D `geo_types` never carries elevation, so positions are
2D), ours → `geo_types` is fallible (`TryFrom`; a position needs at least `x`
and `y`).

```rust
# #[cfg(feature = "geo-types")]
# {
use typed_geojson::Point;

let p: geo_types::Point<f64> = Point::new(vec![-96.8, 32.8]).try_into().unwrap();
assert_eq!(p, geo_types::Point::new(-96.8, 32.8));
# }
```

## TypeScript (`specta` feature)

`Feature<Point, Props>` exports to a TS `Feature<Point, Props>` assignable to
and from `GeoJSON.Feature<GeoJSON.Point, Props>`: zero `tsc` errors, both ways.

```rust
let types = typed_geojson::specta_types();
let ts = specta_typescript::Typescript::default()
    .export(&types, specta_serde::Format)?;
```

Two details make it line up: `bbox` is a tuple union (`[n,n,n,n] | [n,…×6]`),
not `number[]`; each geometry's `"type"` is a string literal. `ts/` holds the
assignability harness, gated in CI by `tsc --noEmit`.

### Compatibility

CI gates the assignability promise across the versions consumers actually
resolve: `@types/geojson` from `7946.0.8` (the `Feature`/`FeatureCollection`
generics era) through latest, on the pinned TypeScript compiler, plus a
non-blocking canary against `typescript@next`.

Known caveat: under `exactOptionalPropertyTypes`, `@types/geojson` declares
optionals as `bbox?: BBox | undefined` while the generated bindings say
`bbox?: Bbox`, so tsc rejects assignment from the native types into the
bindings types. That rendering choice lives in the TypeScript exporter, not
in this crate, so the harness runs without the flag until the exporter grows
an option for it.

## Benchmarks

`cargo bench`, a 1k-point FeatureCollection with typed properties, vs the
untyped `geojson` baseline:

| variant | deserialize | serialize |
| --- | --- | --- |
| untyped `geojson` | ~538 µs | ~176 µs |
| typed properties | ~341 µs | ~158 µs |
| typed properties + our `Geometry` | ~396 µs | ~176 µs |

Typed is ~30% faster to deserialize (typed `properties` skip the untyped JSON
map); the untagged `Geometry` union adds ~16% on reads. Serialize is on par.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your
option. Contributions are dual-licensed as above unless stated otherwise.

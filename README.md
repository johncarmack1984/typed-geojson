# typed-geojson

Strongly-typed [GeoJSON](https://datatracker.ietf.org/doc/html/rfc7946) for Rust:
`Feature<G, P>` and `FeatureCollection<G, P>`, generic over the **G**eometry and
the **P**roperties, layered over the
[georust `geojson`](https://crates.io/crates/geojson) crate.

`geojson::Feature` models `properties` as an untyped
`Option<serde_json::Map<String, Value>>`. Most real data has a *shape* — so this
crate makes it your `P`, and (with the `specta` feature) the whole thing exports
to TypeScript that is **mutually assignable with [`@types/geojson`](https://www.npmjs.com/package/@types/geojson)**.

```rust
use serde::{Deserialize, Serialize};
use typed_geojson::{Feature, Geometry};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Station {
    id: u32,
    name: String,
    temp_c: f64,
}

let raw = r#"{
    "type": "Feature",
    "geometry": { "type": "Point", "coordinates": [-96.8, 32.8] },
    "properties": { "id": 7, "name": "DFW", "temp_c": 31.5 }
}"#;

let feature: Feature<Geometry, Station> = serde_json::from_str(raw).unwrap();
assert_eq!(feature.properties.name, "DFW");

// serializes back to standard GeoJSON
let json = serde_json::to_string(&feature).unwrap();
```

## What you get

- `Feature<G, P>` / `FeatureCollection<G, P>` — same parameter order as
  `@types/geojson`'s `Feature<G, P>`. `G` defaults to the typed [`Geometry`]
  union, `P` to an untyped JSON object.
- A full set of typed geometry types — `Point`, `LineString`, `Polygon`,
  `MultiPoint`, `MultiLineString`, `MultiPolygon`, `GeometryCollection`, and the
  `Geometry` union — each matching its native `@types/geojson` shape.
- serde `Serialize`/`Deserialize` that round-trips to/from spec GeoJSON
  (validates `"type"`, tolerates RFC 7946 foreign members, keeps `id`/`bbox`).
- Bridges to the untyped crate: `geojson::Feature: TryFrom<Feature<Option<Geometry>, P>>`
  and the reverse, plus `geojson::Geometry: TryFrom<Geometry>`.

### Nullability lives in `G`

Per RFC 7946 a Feature's `geometry` is mandatory but may be `null`. Like native
`@types/geojson`, that nullability is expressed through `G`:

```rust
type Located    = Feature<Geometry, Props>;          // geometry: Geometry  (non-null)
type Unlocated  = Feature<Option<Geometry>, Props>;  // geometry: Geometry | null
type PointsOnly = Feature<Point, Props>;             // geometry: Point
```

## TypeScript export (`specta` feature)

Enable the `specta` feature and every type derives [`specta::Type`]. A Rust
function returning, say, `Feature<Point, Props>` exports to a TypeScript
`Feature<Point, Props>` that is assignable **to and from**
`GeoJSON.Feature<GeoJSON.Point, Props>` — with zero `tsc` errors, in both
directions.

```rust
let types = typed_geojson::specta_types();
let ts = specta_typescript::Typescript::default()
    .export(&types, specta_serde::Format)
    .unwrap();
```

The `ts/` directory holds the assignability harness: it imports `@types/geojson`
alongside the generated bindings and asserts mutual assignability across the
geometry × properties × container matrix, gated in CI by `tsc --noEmit`.

A couple of details make the bindings line up exactly with native:

- `bbox` is a **tuple union** (`[number, number, number, number] | [number, …×6]`),
  not `number[]` — so it is assignable to native `BBox`.
- each geometry's `"type"` exports as a **string literal** (`"Point"`), not `string`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

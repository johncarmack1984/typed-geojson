# typed-geojson

Strongly-typed [GeoJSON](https://datatracker.ietf.org/doc/html/rfc7946) for Rust:
`Feature<T>` and `FeatureCollection<T>` with **typed `properties`**, layered over
the [georust `geojson`](https://crates.io/crates/geojson) crate.

`geojson::Feature` models `properties` as an untyped
`Option<serde_json::Map<String, Value>>`. Most real data has a *shape* — so this
crate makes it your `T`, while reusing `geojson::Geometry` for full RFC 7946
geometry fidelity.

```rust
use serde::{Deserialize, Serialize};
use typed_geojson::{Feature, FeatureCollection};

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

let feature: Feature<Station> = serde_json::from_str(raw).unwrap();
assert_eq!(feature.properties.name, "DFW");

// serializes back to standard GeoJSON
let json = serde_json::to_string(&feature).unwrap();
```

## What you get

- `Feature<T>` / `FeatureCollection<T>` — `properties` is your domain type.
- serde `Serialize`/`Deserialize` that round-trips to/from spec GeoJSON
  (validates `"type"`, tolerates RFC 7946 foreign members, keeps `id`/`bbox`).
- `geometry: Option<geojson::Geometry>` — no geometry re-implementation.
- Bridges to the untyped crate: `geojson::Feature: TryFrom<Feature<T>>` and
  `Feature<T>: TryFrom<geojson::Feature>`.

## Bridging to the untyped crate

```rust
let untyped: geojson::Feature = typed_feature.try_into()?;   // T: Serialize
let typed: Feature<Station>   = untyped.try_into()?;          // T: DeserializeOwned
```

## Roadmap

- **`specta` feature** — derive `specta::Type` so `Feature<T>` exports to
  TypeScript as `Feature<T>` with a typed `properties`. The open design
  question is how to represent `geometry` for TS (a typed GeoJSON geometry
  union vs. an opaque value) — being worked out before it lands.
- Typed/owned geometry option (`geo-types` interop).
- `#![no_std]` + `alloc` consideration.

## License

MIT

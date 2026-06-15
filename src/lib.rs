//! Strongly-typed [GeoJSON](https://datatracker.ietf.org/doc/html/rfc7946).
//!
//! The [`geojson`] crate models a Feature's `properties` as an untyped
//! `Option<serde_json::Map<String, Value>>`. This crate adds generics —
//! [`Feature<G, P>`] and [`FeatureCollection<G, P>`] — typed over both the
//! `G`eometry and the `P`roperties, in the same parameter order as
//! `@types/geojson`'s `Feature<G, P>` so the two interoperate.
//!
//! `G` defaults to the typed [`Geometry`] union (which mirrors
//! `@types/geojson`'s `Geometry` and exports to TypeScript via the `specta`
//! feature) and `P` to [`Properties`] (an untyped JSON object, like native
//! `GeoJsonProperties`); pick your own `P` for typed properties:
//!
//! Per RFC 7946 a Feature's geometry is mandatory but may be `null`; like
//! native, that nullability lives in `G` — `Feature<Geometry>` is non-null,
//! `Feature<Option<Geometry>>` (→ TS `Geometry | null`) is the nullable form.
//! [`geojson::Geometry`] interop is available through the [`Feature`] /
//! [`Geometry`] `TryFrom` bridges.
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use typed_geojson::{Feature, Geometry};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! struct Station {
//!     id: u32,
//!     name: String,
//!     temp_c: f64,
//! }
//!
//! let raw = r#"{
//!     "type": "Feature",
//!     "geometry": { "type": "Point", "coordinates": [-96.8, 32.8] },
//!     "properties": { "id": 7, "name": "DFW", "temp_c": 31.5 }
//! }"#;
//!
//! let feature: Feature<Geometry, Station> = serde_json::from_str(raw).unwrap();
//! assert_eq!(feature.properties.name, "DFW");
//!
//! // round-trips back to standard GeoJSON
//! let back: Feature<Geometry, Station> =
//!     serde_json::from_str(&serde_json::to_string(&feature).unwrap()).unwrap();
//! assert_eq!(back, feature);
//! ```

use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

mod geometry;
pub use geometry::*;

/// `specta` export-only marker: a JSON number that maps to the native TS
/// `number`.
///
/// specta-typescript renders Rust `f64` as `number | null` (to model `NaN` /
/// `Infinity`, which serde_json writes as `null`). GeoJSON coordinates and
/// bboxes are finite reals, so for the TS bindings we override those numeric
/// fields to this marker — which renders as a plain `number` — via
/// `#[specta(type = …)]`. serde and the Rust API keep the real `f64`.
#[cfg(feature = "specta")]
pub(crate) struct TsNumber;

#[cfg(feature = "specta")]
impl specta::Type for TsNumber {
    fn definition(_: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::i32)
    }
}

/// The default, untyped `properties` of a [`Feature`]: a JSON object or `null`
/// (RFC 7946 §3.2). Mirrors `@types/geojson`'s
/// `GeoJsonProperties = { [name: string]: any } | null`.
pub type Properties = Option<serde_json::Map<String, serde_json::Value>>;

/// A GeoJSON bounding box (RFC 7946 §5): a flat array of `2*n` numbers, either
/// 4 (2D, `[west, south, east, north]`) or 6 (3D, with min/max elevation).
///
/// Modeled as a tuple union to match `@types/geojson`'s
/// `BBox = [number, number, number, number] | [number, …×6]`, so it is
/// assignable **to and from** the native type (a plain `number[]` is not).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(untagged)]
pub enum Bbox {
    /// 2D bounding box: `[west, south, east, north]`.
    // specta `type =` can't parse an array type (`[T; N]`), but a tuple renders
    // as the same TS tuple — and matches native `BBox`'s `[number, number, …]`.
    D2(
        #[cfg_attr(feature = "specta", specta(type = (TsNumber, TsNumber, TsNumber, TsNumber)))]
        [f64; 4],
    ),
    /// 3D bounding box: `[west, south, min-elev, east, north, max-elev]`.
    D3(
        #[cfg_attr(feature = "specta", specta(type = (TsNumber, TsNumber, TsNumber, TsNumber, TsNumber, TsNumber)))]
         [f64; 6],
    ),
}

impl From<Bbox> for Vec<f64> {
    fn from(bbox: Bbox) -> Self {
        match bbox {
            Bbox::D2(a) => a.to_vec(),
            Bbox::D3(a) => a.to_vec(),
        }
    }
}

/// Build a [`Bbox`] from a flat `Vec<f64>` (as the untyped [`geojson`] crate
/// stores it), accepting only the RFC 7946 lengths of 4 or 6.
pub(crate) fn bbox_from_vec(v: Vec<f64>) -> Result<Bbox, serde_json::Error> {
    match v.len() {
        4 => Ok(Bbox::D2([v[0], v[1], v[2], v[3]])),
        6 => Ok(Bbox::D3([v[0], v[1], v[2], v[3], v[4], v[5]])),
        n => Err(<serde_json::Error as de::Error>::custom(format!(
            "bbox must have 4 or 6 numbers, found {n}"
        ))),
    }
}

/// A GeoJSON Feature `id` — a string or a number (RFC 7946 §3.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(untagged)]
pub enum Id {
    String(String),
    // serde keeps full numeric fidelity; specta exports it as the native TS
    // `number` (via [`TsNumber`] — `f64` would render as `number | null`).
    Number(#[cfg_attr(feature = "specta", specta(type = TsNumber))] serde_json::Number),
}

/// A GeoJSON `Feature` with typed `G`eometry and `P`roperties.
///
/// `G` defaults to [`geojson::Geometry`] and `P` to [`Properties`], matching
/// `@types/geojson`'s `Feature<G = Geometry, P = GeoJsonProperties>`. Supply
/// your own to pin a specific geometry and/or typed properties.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature<G = Geometry, P = Properties> {
    pub geometry: G,
    pub properties: P,
    pub id: Option<Id>,
    pub bbox: Option<Bbox>,
}

impl<G, P> Feature<G, P> {
    /// A `Feature` with just a geometry and properties (no `id`/`bbox`).
    ///
    /// Nullability lives in `G`: use `Feature::<Geometry, _>::new(geom, …)` for
    /// a required geometry, or `Feature::<Option<Geometry>, _>::new(None, …)`
    /// for the RFC 7946 "unlocated" (null-geometry) case.
    pub fn new(geometry: G, properties: P) -> Self {
        Self {
            geometry,
            properties,
            id: None,
            bbox: None,
        }
    }
}

impl<G: Serialize, P: Serialize> Serialize for Feature<G, P> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut len = 3; // type, geometry, properties are always present
        if self.id.is_some() {
            len += 1;
        }
        if self.bbox.is_some() {
            len += 1;
        }
        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("type", "Feature")?;
        // RFC 7946: `geometry` is mandatory but may be null.
        map.serialize_entry("geometry", &self.geometry)?;
        map.serialize_entry("properties", &self.properties)?;
        if let Some(id) = &self.id {
            map.serialize_entry("id", id)?;
        }
        if let Some(bbox) = &self.bbox {
            map.serialize_entry("bbox", bbox)?;
        }
        map.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum FeatureField {
    Type,
    Geometry,
    Properties,
    Id,
    Bbox,
    #[serde(other)]
    Other,
}

impl<'de, G: Deserialize<'de>, P: Deserialize<'de>> Deserialize<'de> for Feature<G, P> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FeatureVisitor<G, P>(PhantomData<(G, P)>);

        impl<'de, G: Deserialize<'de>, P: Deserialize<'de>> Visitor<'de> for FeatureVisitor<G, P> {
            type Value = Feature<G, P>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a GeoJSON Feature object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Feature<G, P>, M::Error> {
                let mut had_type = false;
                let mut geometry: Option<G> = None;
                let mut properties: Option<P> = None;
                let mut id: Option<Id> = None;
                let mut bbox: Option<Bbox> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        FeatureField::Type => {
                            let ty: String = map.next_value()?;
                            if ty != "Feature" {
                                return Err(de::Error::custom(format!(
                                    "expected `type` to be \"Feature\", found {ty:?}"
                                )));
                            }
                            had_type = true;
                        }
                        FeatureField::Geometry => geometry = Some(map.next_value()?),
                        FeatureField::Properties => properties = Some(map.next_value()?),
                        FeatureField::Id => id = map.next_value()?,
                        FeatureField::Bbox => bbox = map.next_value()?,
                        // Ignore unknown keys (RFC 7946 foreign members).
                        FeatureField::Other => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                if !had_type {
                    return Err(de::Error::missing_field("type"));
                }
                // RFC 7946: the `geometry` member is mandatory (its value may
                // be null, which a nullable `G` such as `Option<_>` accepts).
                Ok(Feature {
                    geometry: geometry.ok_or_else(|| de::Error::missing_field("geometry"))?,
                    properties: properties.ok_or_else(|| de::Error::missing_field("properties"))?,
                    id,
                    bbox,
                })
            }
        }

        deserializer.deserialize_map(FeatureVisitor(PhantomData))
    }
}

/// A GeoJSON `FeatureCollection` of typed features.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureCollection<G = Geometry, P = Properties> {
    pub features: Vec<Feature<G, P>>,
    pub bbox: Option<Bbox>,
}

impl<G, P> FromIterator<Feature<G, P>> for FeatureCollection<G, P> {
    fn from_iter<I: IntoIterator<Item = Feature<G, P>>>(iter: I) -> Self {
        Self {
            features: iter.into_iter().collect(),
            bbox: None,
        }
    }
}

impl<G: Serialize, P: Serialize> Serialize for FeatureCollection<G, P> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut len = 2; // type, features
        if self.bbox.is_some() {
            len += 1;
        }
        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("type", "FeatureCollection")?;
        map.serialize_entry("features", &self.features)?;
        if let Some(bbox) = &self.bbox {
            map.serialize_entry("bbox", bbox)?;
        }
        map.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum CollectionField {
    Type,
    Features,
    Bbox,
    #[serde(other)]
    Other,
}

impl<'de, G: Deserialize<'de>, P: Deserialize<'de>> Deserialize<'de> for FeatureCollection<G, P> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CollectionVisitor<G, P>(PhantomData<(G, P)>);

        impl<'de, G: Deserialize<'de>, P: Deserialize<'de>> Visitor<'de> for CollectionVisitor<G, P> {
            type Value = FeatureCollection<G, P>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a GeoJSON FeatureCollection object")
            }

            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<FeatureCollection<G, P>, M::Error> {
                let mut had_type = false;
                let mut features: Option<Vec<Feature<G, P>>> = None;
                let mut bbox: Option<Bbox> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        CollectionField::Type => {
                            let ty: String = map.next_value()?;
                            if ty != "FeatureCollection" {
                                return Err(de::Error::custom(format!(
                                    "expected `type` to be \"FeatureCollection\", found {ty:?}"
                                )));
                            }
                            had_type = true;
                        }
                        CollectionField::Features => features = Some(map.next_value()?),
                        CollectionField::Bbox => bbox = map.next_value()?,
                        CollectionField::Other => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                if !had_type {
                    return Err(de::Error::missing_field("type"));
                }
                Ok(FeatureCollection {
                    features: features.ok_or_else(|| de::Error::missing_field("features"))?,
                    bbox,
                })
            }
        }

        deserializer.deserialize_map(CollectionVisitor(PhantomData))
    }
}

// --- bridges to/from the untyped `geojson` crate (default geometry only) ------

impl From<Id> for geojson::feature::Id {
    fn from(id: Id) -> Self {
        match id {
            Id::String(s) => geojson::feature::Id::String(s),
            Id::Number(n) => geojson::feature::Id::Number(n),
        }
    }
}

impl From<geojson::feature::Id> for Id {
    fn from(id: geojson::feature::Id) -> Self {
        match id {
            geojson::feature::Id::String(s) => Id::String(s),
            geojson::feature::Id::Number(n) => Id::Number(n),
        }
    }
}

// Bridges to/from the untyped `geojson::Feature`, whose geometry is nullable,
// so the typed side is `Feature<Option<Geometry>, P>`. Geometry crosses via the
// `Geometry` <-> `geojson::Geometry` `TryFrom`s (see `geometry.rs`).
impl<P: Serialize> TryFrom<Feature<Option<Geometry>, P>> for geojson::Feature {
    type Error = serde_json::Error;

    fn try_from(f: Feature<Option<Geometry>, P>) -> Result<Self, Self::Error> {
        let properties = match serde_json::to_value(&f.properties)? {
            serde_json::Value::Object(map) => Some(map),
            serde_json::Value::Null => None,
            _ => {
                return Err(<serde_json::Error as serde::ser::Error>::custom(
                    "Feature properties must serialize to a JSON object or null",
                ));
            }
        };
        Ok(geojson::Feature {
            bbox: f.bbox.map(Into::into),
            geometry: f.geometry.map(geojson::Geometry::try_from).transpose()?,
            id: f.id.map(Into::into),
            properties,
            foreign_members: None,
        })
    }
}

impl<P: serde::de::DeserializeOwned> TryFrom<geojson::Feature> for Feature<Option<Geometry>, P> {
    type Error = serde_json::Error;

    fn try_from(f: geojson::Feature) -> Result<Self, Self::Error> {
        let value = match f.properties {
            Some(map) => serde_json::Value::Object(map),
            None => serde_json::Value::Null,
        };
        Ok(Feature {
            geometry: f.geometry.map(Geometry::try_from).transpose()?,
            properties: serde_json::from_value(value)?,
            id: f.id.map(Into::into),
            bbox: f.bbox.map(bbox_from_vec).transpose()?,
        })
    }
}

/// Register every public typed-geojson type into a [`specta::Types`] collection,
/// ready to hand to a language exporter (e.g. `specta_typescript`).
///
/// The collection contains the generic `Feature<G, P>` / `FeatureCollection<G, P>`
/// plus `Geometry` (and each named geometry), `Id`, and `Bbox` — shaped to be
/// mutually assignable with `@types/geojson`. Requires the `specta` feature.
///
/// ```
/// # #[cfg(feature = "specta")]
/// # {
/// let types = typed_geojson::specta_types();
/// // let ts = specta_typescript::Typescript::default()
/// //     .export(&types, specta_serde::Format)?;
/// # }
/// ```
#[cfg(feature = "specta")]
pub fn specta_types() -> specta::Types {
    // `Feature`/`FeatureCollection` are generic; the concrete `Geometry` args
    // here only satisfy registration — specta emits the generic `<G, P>` form.
    specta::Types::default()
        .register::<__ts::Feature<Geometry, Geometry>>()
        .register::<__ts::FeatureCollection<Geometry, Geometry>>()
        .register::<Geometry>()
        .register::<Id>()
        .register::<Bbox>()
}

// --- specta export shadows ----------------------------------------------------
//
// `Feature` / `FeatureCollection` use *manual* serde and so have no `type`
// field for `#[derive(specta::Type)]` to see. These shadows mirror the exact
// wire shape — including the literal `"type"` tag — and exist only to drive
// TypeScript generation. They are not part of the public API.
#[cfg(feature = "specta")]
#[doc(hidden)]
#[allow(dead_code)]
pub mod __ts {
    use serde::{Deserialize, Serialize};

    use super::{Bbox, Id};

    // These derive serde only so `#[serde(rename)]` (which specta reads for the
    // `type`/container names) is valid — the serde impls are never used.

    /// The `"Feature"` value of a Feature's `type` member.
    #[derive(Serialize, Deserialize, specta::Type)]
    pub enum FeatureType {
        Feature,
    }

    /// The `"FeatureCollection"` value of a collection's `type` member.
    #[derive(Serialize, Deserialize, specta::Type)]
    pub enum FeatureCollectionType {
        FeatureCollection,
    }

    /// A GeoJSON Feature: a geometry `G` and its associated `properties` `P`
    /// (RFC 7946 §3.2).
    #[derive(Serialize, Deserialize, specta::Type)]
    #[serde(rename = "Feature")]
    pub struct Feature<G, P> {
        #[serde(rename = "type")]
        r#type: FeatureType,
        geometry: G,
        properties: P,
        #[specta(type = Id, optional)]
        id: Option<Id>,
        #[specta(type = Bbox, optional)]
        bbox: Option<Bbox>,
    }

    /// A GeoJSON FeatureCollection: a list of `Feature`s (RFC 7946 §3.3).
    #[derive(Serialize, Deserialize, specta::Type)]
    #[serde(rename = "FeatureCollection")]
    pub struct FeatureCollection<G, P> {
        #[serde(rename = "type")]
        r#type: FeatureCollectionType,
        features: Vec<Feature<G, P>>,
        #[specta(type = Bbox, optional)]
        bbox: Option<Bbox>,
    }
}

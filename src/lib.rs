//! Strongly-typed [GeoJSON](https://datatracker.ietf.org/doc/html/rfc7946).
//!
//! The [`geojson`] crate models a Feature's `properties` as an untyped
//! `Option<serde_json::Map<String, Value>>`. This crate adds generics —
//! [`Feature<P, G>`] and [`FeatureCollection<P, G>`] — typed over both the
//! `P`roperties and the `G`eometry, mirroring `@types/geojson`'s
//! `Feature<G, P>` so the two interoperate.
//!
//! `G` defaults to [`geojson::Geometry`] (full RFC 7946 fidelity + easy interop
//! with the georust ecosystem); pick your own `P` for typed properties:
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use typed_geojson::Feature;
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
//! let feature: Feature<Station> = serde_json::from_str(raw).unwrap();
//! assert_eq!(feature.properties.name, "DFW");
//!
//! // round-trips back to standard GeoJSON
//! let back: Feature<Station> =
//!     serde_json::from_str(&serde_json::to_string(&feature).unwrap()).unwrap();
//! assert_eq!(back, feature);
//! ```

use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

pub use geojson::Geometry;

/// A GeoJSON bounding box: `[min..., max...]` (RFC 7946 §5).
pub type Bbox = Vec<f64>;

/// A GeoJSON Feature `id` — a string or a number (RFC 7946 §3.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
    String(String),
    Number(serde_json::Number),
}

/// A GeoJSON `Feature` with typed `P`roperties and `G`eometry.
///
/// `G` defaults to [`geojson::Geometry`]; supply your own to pin a specific
/// geometry (e.g. point-only), the way `@types/geojson`'s `Feature<G, P>` does.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature<P, G = Geometry> {
    pub geometry: Option<G>,
    pub properties: P,
    pub id: Option<Id>,
    pub bbox: Option<Bbox>,
}

impl<P, G> Feature<P, G> {
    /// A `Feature` with just a geometry and properties (no `id`/`bbox`).
    pub fn new(geometry: Option<G>, properties: P) -> Self {
        Self {
            geometry,
            properties,
            id: None,
            bbox: None,
        }
    }
}

impl<P: Serialize, G: Serialize> Serialize for Feature<P, G> {
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

impl<'de, P: Deserialize<'de>, G: Deserialize<'de>> Deserialize<'de> for Feature<P, G> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FeatureVisitor<P, G>(PhantomData<(P, G)>);

        impl<'de, P: Deserialize<'de>, G: Deserialize<'de>> Visitor<'de> for FeatureVisitor<P, G> {
            type Value = Feature<P, G>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a GeoJSON Feature object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Feature<P, G>, M::Error> {
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
                        FeatureField::Geometry => geometry = map.next_value()?,
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
                Ok(Feature {
                    geometry,
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
pub struct FeatureCollection<P, G = Geometry> {
    pub features: Vec<Feature<P, G>>,
    pub bbox: Option<Bbox>,
}

impl<P, G> FromIterator<Feature<P, G>> for FeatureCollection<P, G> {
    fn from_iter<I: IntoIterator<Item = Feature<P, G>>>(iter: I) -> Self {
        Self {
            features: iter.into_iter().collect(),
            bbox: None,
        }
    }
}

impl<P: Serialize, G: Serialize> Serialize for FeatureCollection<P, G> {
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

impl<'de, P: Deserialize<'de>, G: Deserialize<'de>> Deserialize<'de> for FeatureCollection<P, G> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CollectionVisitor<P, G>(PhantomData<(P, G)>);

        impl<'de, P: Deserialize<'de>, G: Deserialize<'de>> Visitor<'de> for CollectionVisitor<P, G> {
            type Value = FeatureCollection<P, G>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a GeoJSON FeatureCollection object")
            }

            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<FeatureCollection<P, G>, M::Error> {
                let mut had_type = false;
                let mut features: Option<Vec<Feature<P, G>>> = None;
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

impl<P: Serialize> TryFrom<Feature<P>> for geojson::Feature {
    type Error = serde_json::Error;

    fn try_from(f: Feature<P>) -> Result<Self, Self::Error> {
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
            bbox: f.bbox,
            geometry: f.geometry,
            id: f.id.map(Into::into),
            properties,
            foreign_members: None,
        })
    }
}

impl<P: serde::de::DeserializeOwned> TryFrom<geojson::Feature> for Feature<P> {
    type Error = serde_json::Error;

    fn try_from(f: geojson::Feature) -> Result<Self, Self::Error> {
        let value = match f.properties {
            Some(map) => serde_json::Value::Object(map),
            None => serde_json::Value::Null,
        };
        Ok(Feature {
            geometry: f.geometry,
            properties: serde_json::from_value(value)?,
            id: f.id.map(Into::into),
            bbox: f.bbox,
        })
    }
}

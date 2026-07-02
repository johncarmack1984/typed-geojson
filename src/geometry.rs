//! GeoJSON geometry types (RFC 7946 §3.1), shaped to match `@types/geojson`
//! exactly so they round-trip through spec GeoJSON **and** export (via the
//! `specta` feature) to TypeScript that is mutually assignable with native
//! `GeoJSON.Point`, `GeoJSON.LineString`, … and the `GeoJSON.Geometry` union.
//!
//! Each geometry is its own named type (so you can pin one, like
//! `Feature<Point, P>`); [`Geometry`] is their union. The `"type"` member is a
//! single-variant enum (e.g. [`PointType`]) so it exports as the **string
//! literal** `"Point"`, not `string`, which is what makes the native types
//! assignable.

use serde::{Deserialize, Serialize};

use crate::Bbox;

/// A GeoJSON position (RFC 7946 §3.1.1): longitude, latitude, and an optional
/// third element (elevation). Modeled as `Vec<f64>` to match `@types/geojson`'s
/// `Position = number[]` (a fixed `[f64; N]` would export to a TS tuple, which
/// is not mutually assignable with `number[]`).
pub type Position = Vec<f64>;

/// Defines one coordinate-based geometry: a single-variant `type` tag enum that
/// exports as the string literal, and the struct itself with an optional bbox.
/// (Docs are static string literals because `specta`'s derive parses `#[doc]`
/// and rejects a `concat!`.)
macro_rules! coord_geometry {
    ($name:ident, $tag:ident, $lit:literal, $coords:ty, $ts_coords:ty) => {
        /// The `"type"` member of a single geometry kind: a string literal,
        /// which is what makes the type assignable to the native geometry.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "specta", derive(specta::Type))]
        pub enum $tag {
            #[serde(rename = $lit)]
            $name,
        }

        /// A GeoJSON geometry object (RFC 7946 §3.1), matching the native
        /// `@types/geojson` interface of the same name.
        // `Serialize` is hand-written (below) so an absent `bbox` is *omitted*,
        // not emitted as `null`. We can't use `#[serde(skip_serializing_if)]`
        // because specta's unified mode rejects it on a `#[derive(Type)]` field.
        #[derive(Clone, Debug, PartialEq, Deserialize)]
        #[cfg_attr(feature = "specta", derive(specta::Type))]
        pub struct $name {
            // `specta` reads `#[serde(rename)]`, so the wire/TS name is `type`.
            #[serde(rename = "type")]
            pub r#type: $tag,
            // serde uses the real `f64` coordinates; specta exports them via the
            // `number`-rendering override (see [`crate::TsNumber`]).
            #[cfg_attr(feature = "specta", specta(type = $ts_coords))]
            pub coordinates: $coords,
            // `type = Bbox` (not `Option<Bbox>`) makes the TS field `bbox?: Bbox`.
            #[cfg_attr(feature = "specta", specta(type = Bbox, optional))]
            pub bbox: Option<Bbox>,
        }

        impl $name {
            /// Construct from coordinates (no `bbox`).
            pub fn new(coordinates: $coords) -> Self {
                Self {
                    r#type: $tag::$name,
                    coordinates,
                    bbox: None,
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(2 + self.bbox.is_some() as usize))?;
                map.serialize_entry("type", $lit)?;
                map.serialize_entry("coordinates", &self.coordinates)?;
                if let Some(bbox) = &self.bbox {
                    map.serialize_entry("bbox", bbox)?;
                }
                map.end()
            }
        }
    };
}

coord_geometry!(Point, PointType, "Point", Position, Vec<crate::TsNumber>);
coord_geometry!(
    MultiPoint,
    MultiPointType,
    "MultiPoint",
    Vec<Position>,
    Vec<Vec<crate::TsNumber>>
);
coord_geometry!(
    LineString,
    LineStringType,
    "LineString",
    Vec<Position>,
    Vec<Vec<crate::TsNumber>>
);
coord_geometry!(
    MultiLineString,
    MultiLineStringType,
    "MultiLineString",
    Vec<Vec<Position>>,
    Vec<Vec<Vec<crate::TsNumber>>>
);
coord_geometry!(
    Polygon,
    PolygonType,
    "Polygon",
    Vec<Vec<Position>>,
    Vec<Vec<Vec<crate::TsNumber>>>
);
coord_geometry!(
    MultiPolygon,
    MultiPolygonType,
    "MultiPolygon",
    Vec<Vec<Vec<Position>>>,
    Vec<Vec<Vec<Vec<crate::TsNumber>>>>
);

/// The `"GeometryCollection"` value of a [`GeometryCollection`]'s `type` member.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum GeometryCollectionType {
    GeometryCollection,
}

/// A GeoJSON GeometryCollection (RFC 7946 §3.1.8): a list of geometries.
// `Serialize` is hand-written (below) to omit an absent `bbox`; see the note
// on the coordinate geometries above.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GeometryCollection {
    #[serde(rename = "type")]
    pub r#type: GeometryCollectionType,
    pub geometries: Vec<Geometry>,
    #[cfg_attr(feature = "specta", specta(type = Bbox, optional))]
    pub bbox: Option<Bbox>,
}

impl GeometryCollection {
    /// A `GeometryCollection` from its member geometries (no bbox).
    pub fn new(geometries: Vec<Geometry>) -> Self {
        Self {
            r#type: GeometryCollectionType::GeometryCollection,
            geometries,
            bbox: None,
        }
    }
}

impl Serialize for GeometryCollection {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2 + self.bbox.is_some() as usize))?;
        map.serialize_entry("type", "GeometryCollection")?;
        map.serialize_entry("geometries", &self.geometries)?;
        if let Some(bbox) = &self.bbox {
            map.serialize_entry("bbox", bbox)?;
        }
        map.end()
    }
}

/// The GeoJSON geometry union (RFC 7946 §3.1): mirrors `@types/geojson`'s
/// `Geometry`. Untagged at the serde layer; each member's own `"type"` literal
/// disambiguates, so it round-trips losslessly and exports to the TS union.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(untagged)]
pub enum Geometry {
    Point(Point),
    MultiPoint(MultiPoint),
    LineString(LineString),
    MultiLineString(MultiLineString),
    Polygon(Polygon),
    MultiPolygon(MultiPolygon),
    GeometryCollection(GeometryCollection),
}

// --- interop with the untyped `geojson` crate's geometry ----------------------
//
// `geojson` models coordinates with its own `Position` newtype (a `TinyVec`)
// and a struct-variant `GeometryValue`, so a field-by-field conversion would be
// verbose and easy to get wrong. Both sides are valid RFC 7946 geometry, so we
// bridge through a serde round-trip: lossless for the geometry types we share,
// and fallible only if the untyped value is out-of-spec (e.g. a bbox that is
// not 4 or 6 numbers).

impl TryFrom<Geometry> for geojson::Geometry {
    type Error = serde_json::Error;

    fn try_from(g: Geometry) -> Result<Self, Self::Error> {
        serde_json::from_value(serde_json::to_value(g)?)
    }
}

impl TryFrom<geojson::Geometry> for Geometry {
    type Error = serde_json::Error;

    fn try_from(g: geojson::Geometry) -> Result<Self, Self::Error> {
        serde_json::from_value(serde_json::to_value(g)?)
    }
}

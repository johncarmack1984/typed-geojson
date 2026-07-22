//! Conversions between our typed geometries and [`geo_types`] (the georust
//! primitive layer that `geo`'s algorithms operate on), behind the `geo-types`
//! feature — mirroring the same feature-gated bridge on the underlying
//! `geojson` crate, and matching its semantics:
//!
//! - **`geo_types` → ours is infallible** ([`From`]). A `geo_types` coordinate
//!   is `(x, y)`, so it becomes a 2-element GeoJSON position; a higher
//!   dimension `geo_types` does not carry is simply not present (exactly as
//!   upstream `geojson` does). `Line` → `LineString`, and `Rect` / `Triangle`
//!   → `Polygon`, since GeoJSON has no native form for those.
//! - **ours → `geo_types` is fallible** ([`TryFrom`]). The geometry *kind*
//!   always lines up (our types are concrete), but a position could be missing
//!   its `x`/`y`, so a position with fewer than two numbers is an error. The
//!   error type is [`serde_json::Error`], the one every bridge in this crate
//!   already reports, so callers see a single error type across all of them.
//!
//! All conversions are on `f64` coordinates (GeoJSON is `f64` per RFC 7946).
//! They operate on the geometry types; a `Feature`'s geometry converts by
//! reaching through its `geometry` field.

use serde::de;

use crate::{
    Geometry, GeometryCollection, LineString, MultiLineString, MultiPoint, MultiPolygon, Point,
    Polygon, Position,
};

// --- geo_types -> ours (infallible) -------------------------------------------

/// A `geo_types` coordinate as a 2D GeoJSON position `[x, y]`.
fn position(coord: geo_types::Coord<f64>) -> Position {
    vec![coord.x, coord.y]
}

/// The vertices of a `geo_types` line string as GeoJSON positions.
fn line_string_positions(ls: geo_types::LineString<f64>) -> Vec<Position> {
    ls.0.into_iter().map(position).collect()
}

/// A `geo_types` polygon as GeoJSON rings (exterior first, then interiors).
/// An empty exterior yields no rings — matching upstream, which avoids `[[]]`.
fn polygon_rings(poly: geo_types::Polygon<f64>) -> Vec<Vec<Position>> {
    let (exterior, interiors) = poly.into_inner();
    let exterior = line_string_positions(exterior);
    if exterior.is_empty() {
        return vec![];
    }
    let mut rings = vec![exterior];
    rings.extend(interiors.into_iter().map(line_string_positions));
    rings
}

impl From<geo_types::Point<f64>> for Point {
    fn from(p: geo_types::Point<f64>) -> Self {
        Point::new(position(p.0))
    }
}

impl From<geo_types::MultiPoint<f64>> for MultiPoint {
    fn from(mp: geo_types::MultiPoint<f64>) -> Self {
        MultiPoint::new(mp.0.into_iter().map(|p| position(p.0)).collect())
    }
}

impl From<geo_types::LineString<f64>> for LineString {
    fn from(ls: geo_types::LineString<f64>) -> Self {
        LineString::new(line_string_positions(ls))
    }
}

impl From<geo_types::MultiLineString<f64>> for MultiLineString {
    fn from(mls: geo_types::MultiLineString<f64>) -> Self {
        MultiLineString::new(mls.0.into_iter().map(line_string_positions).collect())
    }
}

impl From<geo_types::Polygon<f64>> for Polygon {
    fn from(poly: geo_types::Polygon<f64>) -> Self {
        Polygon::new(polygon_rings(poly))
    }
}

impl From<geo_types::MultiPolygon<f64>> for MultiPolygon {
    fn from(mp: geo_types::MultiPolygon<f64>) -> Self {
        MultiPolygon::new(mp.0.into_iter().map(polygon_rings).collect())
    }
}

impl From<geo_types::GeometryCollection<f64>> for GeometryCollection {
    fn from(gc: geo_types::GeometryCollection<f64>) -> Self {
        GeometryCollection::new(gc.0.into_iter().map(Geometry::from).collect())
    }
}

impl From<geo_types::Geometry<f64>> for Geometry {
    fn from(g: geo_types::Geometry<f64>) -> Self {
        use geo_types::Geometry as G;
        match g {
            G::Point(p) => Geometry::Point(p.into()),
            G::MultiPoint(mp) => Geometry::MultiPoint(mp.into()),
            G::LineString(ls) => Geometry::LineString(ls.into()),
            G::MultiLineString(mls) => Geometry::MultiLineString(mls.into()),
            G::Polygon(p) => Geometry::Polygon(p.into()),
            G::MultiPolygon(mp) => Geometry::MultiPolygon(mp.into()),
            G::GeometryCollection(gc) => Geometry::GeometryCollection(gc.into()),
            // GeoJSON has no Line / Rect / Triangle; degrade as upstream does.
            G::Line(l) => {
                Geometry::LineString(LineString::new(vec![position(l.start), position(l.end)]))
            }
            G::Rect(r) => Geometry::Polygon(r.to_polygon().into()),
            G::Triangle(t) => Geometry::Polygon(t.to_polygon().into()),
        }
    }
}

// --- ours -> geo_types (fallible on coordinate arity) -------------------------

fn arity_error(len: usize) -> serde_json::Error {
    <serde_json::Error as de::Error>::custom(format!(
        "geo_types needs at least 2 coordinates (x, y) per position, found {len}"
    ))
}

/// A GeoJSON position as a `geo_types` coordinate, taking the first two numbers
/// (`x`, `y`) and ignoring any third (elevation), like upstream `geojson`.
fn coord(pos: &Position) -> Result<geo_types::Coord<f64>, serde_json::Error> {
    match pos.as_slice() {
        [x, y, ..] => Ok(geo_types::Coord { x: *x, y: *y }),
        _ => Err(arity_error(pos.len())),
    }
}

fn geo_line_string(ring: &[Position]) -> Result<geo_types::LineString<f64>, serde_json::Error> {
    ring.iter()
        .map(coord)
        .collect::<Result<Vec<_>, _>>()
        .map(geo_types::LineString)
}

fn geo_polygon(rings: &[Vec<Position>]) -> Result<geo_types::Polygon<f64>, serde_json::Error> {
    let exterior = match rings.first() {
        Some(ring) => geo_line_string(ring)?,
        None => geo_types::LineString(vec![]),
    };
    let interiors = rings
        .iter()
        .skip(1)
        .map(|ring| geo_line_string(ring))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(geo_types::Polygon::new(exterior, interiors))
}

impl TryFrom<Point> for geo_types::Point<f64> {
    type Error = serde_json::Error;
    fn try_from(p: Point) -> Result<Self, Self::Error> {
        Ok(geo_types::Point(coord(&p.coordinates)?))
    }
}

impl TryFrom<MultiPoint> for geo_types::MultiPoint<f64> {
    type Error = serde_json::Error;
    fn try_from(mp: MultiPoint) -> Result<Self, Self::Error> {
        mp.coordinates
            .iter()
            .map(|c| coord(c).map(geo_types::Point))
            .collect::<Result<Vec<_>, _>>()
            .map(geo_types::MultiPoint)
    }
}

impl TryFrom<LineString> for geo_types::LineString<f64> {
    type Error = serde_json::Error;
    fn try_from(ls: LineString) -> Result<Self, Self::Error> {
        geo_line_string(&ls.coordinates)
    }
}

impl TryFrom<MultiLineString> for geo_types::MultiLineString<f64> {
    type Error = serde_json::Error;
    fn try_from(mls: MultiLineString) -> Result<Self, Self::Error> {
        mls.coordinates
            .iter()
            .map(|ring| geo_line_string(ring))
            .collect::<Result<Vec<_>, _>>()
            .map(geo_types::MultiLineString)
    }
}

impl TryFrom<Polygon> for geo_types::Polygon<f64> {
    type Error = serde_json::Error;
    fn try_from(p: Polygon) -> Result<Self, Self::Error> {
        geo_polygon(&p.coordinates)
    }
}

impl TryFrom<MultiPolygon> for geo_types::MultiPolygon<f64> {
    type Error = serde_json::Error;
    fn try_from(mp: MultiPolygon) -> Result<Self, Self::Error> {
        mp.coordinates
            .iter()
            .map(|poly| geo_polygon(poly))
            .collect::<Result<Vec<_>, _>>()
            .map(geo_types::MultiPolygon)
    }
}

impl TryFrom<GeometryCollection> for geo_types::GeometryCollection<f64> {
    type Error = serde_json::Error;
    fn try_from(gc: GeometryCollection) -> Result<Self, Self::Error> {
        gc.geometries
            .into_iter()
            .map(geo_types::Geometry::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(geo_types::GeometryCollection)
    }
}

impl TryFrom<Geometry> for geo_types::Geometry<f64> {
    type Error = serde_json::Error;
    fn try_from(g: Geometry) -> Result<Self, Self::Error> {
        Ok(match g {
            Geometry::Point(p) => geo_types::Geometry::Point(p.try_into()?),
            Geometry::MultiPoint(mp) => geo_types::Geometry::MultiPoint(mp.try_into()?),
            Geometry::LineString(ls) => geo_types::Geometry::LineString(ls.try_into()?),
            Geometry::MultiLineString(mls) => geo_types::Geometry::MultiLineString(mls.try_into()?),
            Geometry::Polygon(p) => geo_types::Geometry::Polygon(p.try_into()?),
            Geometry::MultiPolygon(mp) => geo_types::Geometry::MultiPolygon(mp.try_into()?),
            Geometry::GeometryCollection(gc) => {
                geo_types::Geometry::GeometryCollection(gc.try_into()?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_round_trips_through_geo_types() {
        let ours = Point::new(vec![-96.8, 32.8]);
        let geo: geo_types::Point<f64> = ours.clone().try_into().unwrap();
        assert_eq!(geo, geo_types::Point::new(-96.8, 32.8));
        // geo -> ours reproduces the 2D position.
        assert_eq!(Point::from(geo), ours);
    }

    #[test]
    fn elevation_is_dropped_like_upstream() {
        // A 3D position converts (x, y kept; z dropped), matching `geojson`.
        let p3d = Point::new(vec![1.0, 2.0, 3.0]);
        let geo: geo_types::Point<f64> = p3d.try_into().unwrap();
        assert_eq!(geo, geo_types::Point::new(1.0, 2.0));
    }

    #[test]
    fn short_position_is_an_error() {
        let bad = Point::new(vec![1.0]);
        let err = geo_types::Point::<f64>::try_from(bad).unwrap_err();
        assert!(err.to_string().contains("at least 2"));
    }

    #[test]
    fn polygon_rings_survive_both_directions() {
        // A square with a triangular hole: exterior ring + one interior ring.
        let ours = Polygon::new(vec![
            vec![
                vec![0.0, 0.0],
                vec![4.0, 0.0],
                vec![4.0, 4.0],
                vec![0.0, 4.0],
                vec![0.0, 0.0],
            ],
            vec![
                vec![1.0, 1.0],
                vec![2.0, 1.0],
                vec![1.0, 2.0],
                vec![1.0, 1.0],
            ],
        ]);
        let geo: geo_types::Polygon<f64> = ours.clone().try_into().unwrap();
        assert_eq!(geo.interiors().len(), 1);
        assert_eq!(Polygon::from(geo), ours);
    }

    #[test]
    fn geo_line_and_triangle_degrade_to_geojson_shapes() {
        let line = geo_types::Line::new(
            geo_types::Coord { x: 0.0, y: 0.0 },
            geo_types::Coord { x: 1.0, y: 1.0 },
        );
        assert!(matches!(
            Geometry::from(geo_types::Geometry::Line(line)),
            Geometry::LineString(_)
        ));

        let tri = geo_types::Triangle::new(
            geo_types::Coord { x: 0.0, y: 0.0 },
            geo_types::Coord { x: 2.0, y: 0.0 },
            geo_types::Coord { x: 1.0, y: 1.0 },
        );
        assert!(matches!(
            Geometry::from(geo_types::Geometry::Triangle(tri)),
            Geometry::Polygon(_)
        ));
    }

    #[test]
    fn geometry_union_round_trips() {
        let ours = Geometry::MultiPoint(MultiPoint::new(vec![vec![0.0, 0.0], vec![1.0, 1.0]]));
        let geo: geo_types::Geometry<f64> = ours.clone().try_into().unwrap();
        assert_eq!(Geometry::from(geo), ours);
    }
}

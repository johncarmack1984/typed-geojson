use serde::{Deserialize, Serialize};
use typed_geojson::{
    Bbox, Feature, FeatureCollection, Geometry, GeometryCollection, Id, LineString, MultiPoint,
    Point,
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Alert {
    event: String,
    severity: u8,
}

const FC: &str = r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "id": "urn:oid:1",
      "geometry": { "type": "Point", "coordinates": [-96.8, 32.8] },
      "properties": { "event": "Tornado Warning", "severity": 3 }
    },
    {
      "type": "Feature",
      "geometry": null,
      "properties": { "event": "Flood Watch", "severity": 1 }
    }
  ]
}"#;

#[test]
fn feature_collection_round_trips() {
    // Mixed null/non-null geometry => the nullable form `Option<Geometry>`.
    let fc: FeatureCollection<Option<Geometry>, Alert> = serde_json::from_str(FC).unwrap();
    assert_eq!(fc.features.len(), 2);
    assert_eq!(fc.features[0].properties.event, "Tornado Warning");
    assert_eq!(fc.features[0].id, Some(Id::String("urn:oid:1".into())));
    assert!(fc.features[1].geometry.is_none());

    let serialized = serde_json::to_string(&fc).unwrap();
    let reparsed: FeatureCollection<Option<Geometry>, Alert> =
        serde_json::from_str(&serialized).unwrap();
    assert_eq!(reparsed, fc);
}

#[test]
fn wrong_type_is_rejected() {
    let bad = r#"{ "type": "Point", "properties": { "event": "x", "severity": 0 } }"#;
    let err = serde_json::from_str::<Feature<Geometry, Alert>>(bad).unwrap_err();
    assert!(err.to_string().contains("Feature"));
}

#[test]
fn bridges_to_and_from_untyped_geojson() {
    let typed: Feature<Option<Geometry>, Alert> = serde_json::from_str(
        r#"{ "type": "Feature", "geometry": null,
             "properties": { "event": "Heat Advisory", "severity": 2 } }"#,
    )
    .unwrap();

    // typed -> untyped geojson::Feature -> typed
    let untyped: geojson::Feature = typed.clone().try_into().unwrap();
    assert_eq!(
        untyped.properties.as_ref().unwrap()["event"],
        serde_json::json!("Heat Advisory")
    );

    let back: Feature<Option<Geometry>, Alert> = untyped.try_into().unwrap();
    assert_eq!(back, typed);
}

#[test]
fn numeric_id_preserved() {
    let f: Feature<Option<Geometry>, Alert> = serde_json::from_str(
        r#"{ "type": "Feature", "id": 42, "geometry": null,
             "properties": { "event": "x", "severity": 0 } }"#,
    )
    .unwrap();
    assert_eq!(f.id, Some(Id::Number(42.into())));
}

// A user-supplied geometry type: the generic `G` lets you pin geometry the way
// `@types/geojson`'s `Feature<G, P>` does (a preview of the planned
// specta-friendly geometry types — see .claude/NEXT-STEPS.md).
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "type")]
enum PointGeom {
    Point { coordinates: [f64; 2] },
}

#[test]
fn typed_geometry_param() {
    let raw = r#"{ "type": "Feature",
                   "geometry": { "type": "Point", "coordinates": [-96.8, 32.8] },
                   "properties": { "event": "x", "severity": 0 } }"#;
    // `G` is required (non-null) here, so `geometry` is a `PointGeom`, not an
    // `Option` — exactly how native `Feature<Point, P>` has `geometry: Point`.
    let f: Feature<PointGeom, Alert> = serde_json::from_str(raw).unwrap();
    assert_eq!(
        f.geometry,
        PointGeom::Point {
            coordinates: [-96.8, 32.8]
        }
    );

    let back: Feature<PointGeom, Alert> =
        serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
    assert_eq!(back, f);
}

#[test]
fn bbox_2d_and_3d_round_trip() {
    // 2D bbox -> Bbox::D2; serializes back to a flat 4-element array.
    let raw2d = r#"{ "type": "Feature", "geometry": null,
                     "bbox": [-10.0, -20.0, 10.0, 20.0],
                     "properties": { "event": "x", "severity": 0 } }"#;
    let f2: Feature<Option<Geometry>, Alert> = serde_json::from_str(raw2d).unwrap();
    assert_eq!(f2.bbox, Some(Bbox::D2([-10.0, -20.0, 10.0, 20.0])));
    assert!(
        serde_json::to_string(&f2)
            .unwrap()
            .contains(r#""bbox":[-10.0,-20.0,10.0,20.0]"#)
    );

    // 6 numbers -> Bbox::D3.
    let raw3d = r#"{ "type": "Feature", "geometry": null,
                     "bbox": [-1.0, -2.0, -3.0, 1.0, 2.0, 3.0],
                     "properties": { "event": "x", "severity": 0 } }"#;
    let f3: Feature<Option<Geometry>, Alert> = serde_json::from_str(raw3d).unwrap();
    assert_eq!(f3.bbox, Some(Bbox::D3([-1.0, -2.0, -3.0, 1.0, 2.0, 3.0])));

    // A non-{4,6} length is rejected (geometry is valid here, isolating bbox).
    let bad = r#"{ "type": "Feature", "geometry": null, "bbox": [0.0, 1.0, 2.0],
                   "properties": { "event": "x", "severity": 0 } }"#;
    assert!(serde_json::from_str::<Feature<Option<Geometry>, Alert>>(bad).is_err());
}

#[test]
fn bbox_survives_the_untyped_bridge() {
    let typed: Feature<Option<Geometry>, Alert> = serde_json::from_str(
        r#"{ "type": "Feature", "geometry": null,
             "bbox": [-1.0, -2.0, 3.0, 4.0],
             "properties": { "event": "x", "severity": 0 } }"#,
    )
    .unwrap();

    let untyped: geojson::Feature = typed.clone().try_into().unwrap();
    assert_eq!(
        untyped.bbox.as_deref(),
        Some([-1.0, -2.0, 3.0, 4.0].as_slice())
    );

    let back: Feature<Option<Geometry>, Alert> = untyped.try_into().unwrap();
    assert_eq!(back, typed);
}

// --- our own geometry types ---------------------------------------------------

#[test]
fn each_geometry_round_trips_through_its_named_type() {
    // Point: coordinates is a bare Position (number[]).
    let p: Point =
        serde_json::from_str(r#"{ "type": "Point", "coordinates": [1.0, 2.0] }"#).unwrap();
    assert_eq!(p.coordinates, vec![1.0, 2.0]);
    assert_eq!(p, Point::new(vec![1.0, 2.0]));

    // LineString and MultiPoint share a coordinate shape (Vec<Position>); the
    // `"type"` literal is what keeps them distinct.
    let ls: LineString = serde_json::from_str(
        r#"{ "type": "LineString", "coordinates": [[0.0, 0.0], [1.0, 1.0]] }"#,
    )
    .unwrap();
    assert_eq!(ls.coordinates.len(), 2);
    assert!(serde_json::from_str::<MultiPoint>(&serde_json::to_string(&ls).unwrap()).is_err());

    // A Point JSON will not deserialize as a LineString, and vice versa.
    assert!(
        serde_json::from_str::<Point>(r#"{ "type": "LineString", "coordinates": [] }"#).is_err()
    );
}

#[test]
fn geometry_union_disambiguates_by_type_literal() {
    // MultiPoint and LineString are structurally identical (Vec<Position>); the
    // untagged `Geometry` union must still pick the right arm via `"type"`.
    let mp: Geometry =
        serde_json::from_str(r#"{ "type": "MultiPoint", "coordinates": [[1.0, 2.0]] }"#).unwrap();
    assert!(matches!(mp, Geometry::MultiPoint(_)));

    let ls: Geometry =
        serde_json::from_str(r#"{ "type": "LineString", "coordinates": [[1.0, 2.0]] }"#).unwrap();
    assert!(matches!(ls, Geometry::LineString(_)));

    let gc: Geometry = serde_json::from_str(
        r#"{ "type": "GeometryCollection",
             "geometries": [{ "type": "Point", "coordinates": [0.0, 0.0] }] }"#,
    )
    .unwrap();
    assert!(matches!(gc, Geometry::GeometryCollection(_)));

    // Round-trips back to the same JSON shape.
    let back: Geometry = serde_json::from_str(&serde_json::to_string(&mp).unwrap()).unwrap();
    assert_eq!(back, mp);
}

#[test]
fn geometry_bridges_to_and_from_geojson() {
    let line = Geometry::LineString(LineString::new(vec![vec![0.0, 0.0], vec![3.0, 4.0]]));

    // our Geometry -> geojson::Geometry -> our Geometry
    let untyped: geojson::Geometry = line.clone().try_into().unwrap();
    let back: Geometry = untyped.try_into().unwrap();
    assert_eq!(back, line);

    // A nested GeometryCollection survives the bridge too.
    let nested = Geometry::GeometryCollection(GeometryCollection::new(vec![
        Geometry::Point(Point::new(vec![1.0, 1.0])),
        Geometry::LineString(LineString::new(vec![vec![0.0, 0.0], vec![1.0, 1.0]])),
    ]));
    let round: Geometry = geojson::Geometry::try_from(nested.clone())
        .and_then(Geometry::try_from)
        .unwrap();
    assert_eq!(round, nested);
}

#[test]
fn geometry_omits_absent_bbox() {
    // An absent bbox is omitted, not written as `"bbox":null` (RFC 7946: a bbox
    // member, when present, must be an array).
    let p = Point::new(vec![1.0, 2.0]);
    assert_eq!(
        serde_json::to_string(&p).unwrap(),
        r#"{"type":"Point","coordinates":[1.0,2.0]}"#
    );

    // A present bbox is written.
    let p = Point {
        bbox: Some(Bbox::D2([0.0, 0.0, 1.0, 1.0])),
        ..Point::new(vec![1.0, 2.0])
    };
    assert!(
        serde_json::to_string(&p)
            .unwrap()
            .contains(r#""bbox":[0.0,0.0,1.0,1.0]"#)
    );

    // Holds through the union and a GeometryCollection.
    let gc = Geometry::GeometryCollection(GeometryCollection::new(vec![Geometry::Point(
        Point::new(vec![3.0, 4.0]),
    )]));
    assert_eq!(
        serde_json::to_string(&gc).unwrap(),
        r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[3.0,4.0]}]}"#
    );
}

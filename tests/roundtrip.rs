use serde::{Deserialize, Serialize};
use typed_geojson::{Feature, FeatureCollection, Id};

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
    let fc: FeatureCollection<Alert> = serde_json::from_str(FC).unwrap();
    assert_eq!(fc.features.len(), 2);
    assert_eq!(fc.features[0].properties.event, "Tornado Warning");
    assert_eq!(fc.features[0].id, Some(Id::String("urn:oid:1".into())));
    assert!(fc.features[1].geometry.is_none());

    let serialized = serde_json::to_string(&fc).unwrap();
    let reparsed: FeatureCollection<Alert> = serde_json::from_str(&serialized).unwrap();
    assert_eq!(reparsed, fc);
}

#[test]
fn wrong_type_is_rejected() {
    let bad = r#"{ "type": "Point", "properties": { "event": "x", "severity": 0 } }"#;
    let err = serde_json::from_str::<Feature<Alert>>(bad).unwrap_err();
    assert!(err.to_string().contains("Feature"));
}

#[test]
fn bridges_to_and_from_untyped_geojson() {
    let typed: Feature<Alert> = serde_json::from_str(
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

    let back: Feature<Alert> = untyped.try_into().unwrap();
    assert_eq!(back, typed);
}

#[test]
fn numeric_id_preserved() {
    let f: Feature<Alert> = serde_json::from_str(
        r#"{ "type": "Feature", "id": 42, "geometry": null,
             "properties": { "event": "x", "severity": 0 } }"#,
    )
    .unwrap();
    assert_eq!(f.id, Some(Id::Number(42.into())));
}

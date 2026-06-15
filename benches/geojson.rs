//! Serialize / deserialize benchmarks: the typed path vs the untyped `geojson`
//! baseline, on a FeatureCollection of point features with typed properties.
//!
//! Three variants isolate where any cost lives:
//! - `untyped` — `geojson::FeatureCollection`, untyped properties (a JSON map).
//! - `geojson_geometry` — typed `Props`, geojson's tagged geometry.
//! - `our_geometry` — typed `Props` + our untagged `Geometry` union.
//!
//! Run with `cargo bench`.

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};
use typed_geojson::{FeatureCollection, Geometry};

#[derive(Serialize, Deserialize, Clone)]
struct Props {
    name: String,
    population: u64,
    area_km2: f64,
}

/// A FeatureCollection of `n` point features as spec GeoJSON text.
fn sample_json(n: usize) -> String {
    let mut features = Vec::with_capacity(n);
    for i in 0..n {
        let lon = -180.0 + (i as f64 * 0.31) % 360.0;
        let lat = -90.0 + (i as f64 * 0.17) % 180.0;
        features.push(format!(
            r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[{lon:.5},{lat:.5}]}},"properties":{{"name":"city {i}","population":{},"area_km2":{:.2}}}}}"#,
            10_000 + i as u64 * 37,
            12.5 + i as f64,
        ));
    }
    format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features.join(",")
    )
}

fn benches(c: &mut Criterion) {
    let n = 1_000;
    let json = sample_json(n);

    let mut de = c.benchmark_group("deserialize");
    de.throughput(Throughput::Bytes(json.len() as u64));
    de.bench_function("untyped", |b| {
        b.iter(|| {
            let fc: geojson::FeatureCollection = serde_json::from_str(black_box(&json)).unwrap();
            black_box(fc);
        })
    });
    de.bench_function("geojson_geometry", |b| {
        b.iter(|| {
            let fc: FeatureCollection<geojson::Geometry, Props> =
                serde_json::from_str(black_box(&json)).unwrap();
            black_box(fc);
        })
    });
    de.bench_function("our_geometry", |b| {
        b.iter(|| {
            let fc: FeatureCollection<Geometry, Props> =
                serde_json::from_str(black_box(&json)).unwrap();
            black_box(fc);
        })
    });
    de.finish();

    let untyped: geojson::FeatureCollection = serde_json::from_str(&json).unwrap();
    let typed_gj: FeatureCollection<geojson::Geometry, Props> =
        serde_json::from_str(&json).unwrap();
    let typed_our: FeatureCollection<Geometry, Props> = serde_json::from_str(&json).unwrap();

    let mut ser = c.benchmark_group("serialize");
    ser.throughput(Throughput::Bytes(json.len() as u64));
    ser.bench_function("untyped", |b| {
        b.iter(|| black_box(serde_json::to_string(black_box(&untyped)).unwrap()))
    });
    ser.bench_function("geojson_geometry", |b| {
        b.iter(|| black_box(serde_json::to_string(black_box(&typed_gj)).unwrap()))
    });
    ser.bench_function("our_geometry", |b| {
        b.iter(|| black_box(serde_json::to_string(black_box(&typed_our)).unwrap()))
    });
    ser.finish();
}

criterion_group! {
    name = benches_group;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(3))
        .warm_up_time(Duration::from_millis(500))
        .sample_size(60);
    targets = benches
}
criterion_main!(benches_group);

//! Generate `ts/bindings.ts` from the specta types.
//!
//! Run with: `cargo run --example export_bindings --features specta`
//! CI regenerates and `git diff --exit-code`s to catch drift.

use specta_typescript::Typescript;

fn main() {
    let types = typed_geojson::specta_types();
    let ts = Typescript::default()
        .export(&types, specta_serde::Format)
        .expect("specta export failed");

    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/ts/bindings.ts");
    std::fs::write(out, &ts).expect("failed to write ts/bindings.ts");
    eprintln!("wrote {out} ({} bytes)", ts.len());
}

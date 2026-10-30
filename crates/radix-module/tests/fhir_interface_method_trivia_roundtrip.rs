#![cfg(feature = "hir-fhir")]

//! FHIR round trip of a commented interface method.
//!
//! A `#` comment above an `implendum` method is owned by that method's
//! DefId in the presentation sidecar. The FHIR loader checks every owner
//! DefId against its definition registry, so the registry must know
//! interface-method DefIds or the artifact's own loader rejects it.

use radix_module::driver::{AnalyzeRequest, Config, Session, analyze_source};
use radix_module::hir::artifact::load_fhir;

const SOURCE: &str = "implendum Forma {\n  # before method\n  functio ping() → vacuum\n}\n";

#[test]
fn commented_interface_method_survives_fhir_round_trip() {
    let session = Session::new(Config::default().with_latin_locale().with_dev_locale_root());
    let unit = analyze_source(&session, "iface-trivia", SOURCE, AnalyzeRequest::default())
        .unwrap_or_else(|e| panic!("analyze_source failed: {e:?}"));

    let artifact = radix_module::hir::serialize::from_analyzed(&unit, "test_hash", "test.faber");
    let bytes = postcard::to_allocvec(&artifact).expect("postcard serialize should succeed");
    let loaded = load_fhir(&bytes).unwrap_or_else(|e| panic!("load_fhir rejected its own artifact: {e}"));

    assert_eq!(
        loaded.presentation, artifact.presentation,
        "presentation sidecar lost or mutated across serialize→load"
    );
}

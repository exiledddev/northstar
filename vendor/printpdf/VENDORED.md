# printpdf 0.7.0, vendored

This is [printpdf](https://github.com/fschutt/printpdf) 0.7.0 exactly as
published on crates.io (MIT, see `LICENSE`), with two changes to the code. They let
it build for `wasm32-unknown-unknown`, which the Northstar web app needs:

1. `src/document_info.rs`: 0.7.0's `to_pdf_time_stamp_metadata` calls
   `offset()` and `u8::from(month())`. The wasm32 stand-in date types in
   `src/date.rs` have neither, so the crate does not compile for the browser.
   A browser-only twin of the function writes the same format as UTC. The
   desktop build compiles the original function, unchanged.
2. `src/lib.rs`: `ICC_PROFILE_ECI_V2` is an empty slice instead of
   `include_bytes!("../assets/CoatedFOGRA39.icc")`. That 650 KB CMYK profile
   is only embedded for PDF/X conformance levels. Northstar uses the default
   custom conformance, which has `requires_icc_profile: false`, so its PDFs
   never contained the profile.

A third change only touches lints: `#![allow(warnings)]` at the top of
`src/lib.rs`. A path dependency's warnings are otherwise printed with
Northstar's own, and they belong to upstream.

Examples, badges, the docs.rs metadata and unused assets were left out of
`Cargo.toml` and the folder. Nothing else differs from the published crate.

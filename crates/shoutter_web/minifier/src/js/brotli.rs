// Rust's brotli libraries are very mysterious so
// we decided to use JS library here as well.

use js_sys::Uint8Array;
use wasm_bindgen::prelude::wasm_bindgen;

pub fn compress(src: &[u8]) -> Vec<u8> {
    #[wasm_bindgen(module = "brotli")]
    extern "C" {
        fn compress(src: &[u8]) -> Uint8Array;
    }

    compress(src).to_vec()
}

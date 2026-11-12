mod api;
pub mod auto;
pub mod core;
pub mod graph;
pub mod ops;
pub mod score;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

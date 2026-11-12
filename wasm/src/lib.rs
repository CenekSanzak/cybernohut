mod api;
pub mod core;
pub mod graph;
pub mod ops;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

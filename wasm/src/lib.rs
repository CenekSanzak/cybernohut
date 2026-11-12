mod api;
pub mod core;
pub mod ops;
mod string_ops;
mod utils;

use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
    set_panic_hook();
}

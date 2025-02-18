mod utils;
mod image_ops;
mod string_ops;
mod cipher;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
    set_panic_hook();
}

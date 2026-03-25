#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (
        web_sys::console::log_1(&format!($($t)*).into())
    )
}

#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => {{
        web_sys::console::error_1(&format!($($t)*).into());
        panic!("Aborting due to error...");
    }}
}

mod cpu;
mod emulator;

use wasm_bindgen::prelude::*;
use std::cell::RefCell;

use emulator::Emulator;

thread_local! {
    static EMULATOR: RefCell<Option<Emulator>> = RefCell::new(None);
}

#[wasm_bindgen]
pub fn init_emulator(rom: Vec<u8>) {
    EMULATOR.with(|e| {
        *e.borrow_mut() = Some(Emulator::new(rom));
    });
}

#[wasm_bindgen]
pub fn execute() -> u8 {
    EMULATOR.with(|e| {
        match e.borrow_mut().as_mut() {
            Some(em) => em.execute(),
            _ => 0
        }
    })
}

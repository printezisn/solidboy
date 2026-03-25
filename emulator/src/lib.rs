use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_namespace = window)]
extern "C" {
    fn append_emulator_message(msg: &str);
    fn set_emulator_message(msg: &str);
    fn set_emulator_error(err: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (
        crate::append_emulator_message(&format!($($t)*));
    )
}

#[macro_export]
macro_rules! console_render {
    ($($t:tt)*) => (
        crate::set_emulator_message(&format!($($t)*));
    )
}

#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => {{
        crate::set_emulator_error(&format!($($t)*));
        panic!("Aborting due to error...");
    }}
}

mod cpu;
mod emulator;

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

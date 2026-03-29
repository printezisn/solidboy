use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn emulator_console_log(msg: &str);
    fn emulator_console_error(err: &str);
    fn render_frame_buffer(frame_buffer_ptr: *const u8, length: usize);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (
        crate::emulator_console_log(&format!($($t)*));
    )
}

#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => {{
        crate::emulator_console_error(&format!($($t)*));
        panic!("Aborting due to error...");
    }}
}

#[macro_export]
macro_rules! render_frame_buffer {
    ($($t:tt)*) => {{
        crate::render_frame_buffer($($t)*);
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
    EMULATOR.with(|e| match e.borrow_mut().as_mut() {
        Some(em) => em.execute(),
        _ => 0,
    })
}

#[wasm_bindgen]
pub fn emulator_memory() -> JsValue {
    wasm_bindgen::memory()
}

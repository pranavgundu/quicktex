#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[rustfmt::skip]
mod data;
mod emit;
mod glyph_data;
mod packages;
mod tex;

#[cfg(not(test))]
use core::arch::wasm32;
#[cfg(not(test))]
use core::mem::MaybeUninit;
#[cfg(not(test))]
use tex::Engine;

// Document assets and paragraph wrapping allocate temporary buffers with
// overlapping lifetimes. Reclaim them even when they are freed out of order.
#[cfg(not(test))]
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    wasm32::unreachable()
}

#[cfg(not(test))]
static mut ENGINE: MaybeUninit<Engine> = MaybeUninit::uninit();
#[cfg(not(test))]
static mut READY: bool = false;

#[cfg(not(test))]
#[inline(always)]
fn engine() -> &'static mut Engine {
    unsafe {
        if !READY {
            (&raw mut ENGINE).write(MaybeUninit::new(Engine::new()));
            READY = true;
        }
        (*(&raw mut ENGINE)).assume_init_mut()
    }
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_input(len: usize) -> *mut u8 {
    engine().input_buffer(len)
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_render(len: usize, display: u32) -> i32 {
    engine().render(len, display != 0)
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_document(len: usize) -> i32 {
    engine().render_document(len)
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_page(page: usize) -> i32 {
    engine().document_page(page)
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_pdf() -> i32 {
    engine().document_pdf()
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_use(len: usize) -> i32 {
    engine().use_session_package(len)
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_names() -> i32 {
    engine().command_list()
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_warnings() -> i32 {
    engine().warning_list()
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_output() -> *const u8 {
    engine().output_ptr()
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_error_pos() -> usize {
    engine().error_pos()
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_glyph(len: usize) -> i32 {
    engine().register_glyph(len)
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_files_clear() {
    engine().clear_files()
}
#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn qt_file(name: usize, len: usize) -> i32 {
    engine().register_file(name, len)
}

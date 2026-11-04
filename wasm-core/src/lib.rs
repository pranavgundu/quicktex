#![no_std]

extern crate alloc;

mod data;
mod emit;
mod packages;
mod tex;

use core::alloc::{GlobalAlloc, Layout};
use core::arch::wasm32;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use tex::Engine;

struct Bump {
    top: UnsafeCell<usize>,
    end: UnsafeCell<usize>,
}

unsafe impl Sync for Bump {}

unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        unsafe {
            let top = self.top.get();
            let end = self.end.get();
            if *end == 0 {
                unsafe extern "C" {
                    static __heap_base: u8;
                }
                *top = &raw const __heap_base as usize;
                *end = wasm32::memory_size(0) << 16;
            }
            let start = (*top + l.align() - 1) & !(l.align() - 1);
            let next = start + l.size();
            if next > *end {
                let need = (next - *end + 0xffff) >> 16;
                let pages = need.max(wasm32::memory_size(0));
                if wasm32::memory_grow(0, pages) == usize::MAX
                    && wasm32::memory_grow(0, need) == usize::MAX
                {
                    return core::ptr::null_mut();
                }
                *end = wasm32::memory_size(0) << 16;
            }
            *top = next;
            start as *mut u8
        }
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe {
            let top = self.top.get();
            if p as usize + l.size() == *top {
                *top = p as usize;
            }
        }
    }

    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        unsafe {
            let top = self.top.get();
            if p as usize + l.size() == *top {
                let next = p as usize + n;
                if next <= *self.end.get() {
                    *top = next;
                    return p;
                }
            }
            let q = self.alloc(Layout::from_size_align_unchecked(n, l.align()));
            if !q.is_null() {
                core::ptr::copy_nonoverlapping(p, q, l.size().min(n));
                self.dealloc(p, l);
            }
            q
        }
    }
}

#[global_allocator]
static ALLOC: Bump = Bump {
    top: UnsafeCell::new(0),
    end: UnsafeCell::new(0),
};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    wasm32::unreachable()
}

static mut ENGINE: MaybeUninit<Engine> = MaybeUninit::uninit();
static mut READY: bool = false;

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

#[unsafe(no_mangle)]
extern "C" fn qt_input(len: usize) -> *mut u8 {
    engine().input_buffer(len)
}

#[unsafe(no_mangle)]
extern "C" fn qt_render(len: usize, display: u32) -> i32 {
    engine().render(len, display != 0)
}

#[unsafe(no_mangle)]
extern "C" fn qt_use(len: usize) -> i32 {
    engine().use_session_package(len)
}

#[unsafe(no_mangle)]
extern "C" fn qt_output() -> *const u8 {
    engine().output_ptr()
}

#[unsafe(no_mangle)]
extern "C" fn qt_error_pos() -> usize {
    engine().error_pos()
}

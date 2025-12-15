//! Thread-local scratch arena for zero-GC temporary allocations.
//!
//! Used by SmartBoundFunction to convert Lua strings to C strings
//! without triggering garbage collection.
//!
//! # Safety
//! - All allocations are invalidated after `reset()` is called
//! - Pointers must not be used after the function call returns

use std::cell::RefCell;

/// Default scratch arena size: 64KB
const DEFAULT_SCRATCH_SIZE: usize = 64 * 1024;

thread_local! {
    /// Thread-local scratch arena for temporary allocations
    pub static SCRATCH_ARENA: RefCell<ScratchArena> = RefCell::new(ScratchArena::new(DEFAULT_SCRATCH_SIZE));
}

/// A bump allocator for temporary allocations during FFI calls.
///
/// The arena pre-allocates a contiguous buffer and hands out pointers
/// by incrementing an offset. After the FFI call completes, `reset()`
/// is called to reclaim all memory instantly (O(1)).
pub struct ScratchArena {
    buffer: Vec<u8>,
    offset: usize,
    high_water_mark: usize,
}

impl ScratchArena {
    /// Create a new scratch arena with the given capacity.
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity],
            offset: 0,
            high_water_mark: 0,
        }
    }

    /// Allocate `size` bytes with given alignment.
    ///
    /// Returns a raw pointer to the allocated memory.
    /// Returns `None` if the arena is exhausted.
    #[inline]
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        // Align the current offset
        let align = align.max(1);
        let aligned_offset = (self.offset + align - 1) & !(align - 1);
        let new_offset = aligned_offset + size;

        if new_offset > self.buffer.len() {
            return None; // Out of space
        }

        self.offset = new_offset;
        self.high_water_mark = self.high_water_mark.max(new_offset);

        Some(unsafe { self.buffer.as_mut_ptr().add(aligned_offset) })
    }

    /// Allocate a null-terminated C string from a byte slice.
    ///
    /// Returns a pointer to the string, or `None` if out of space.
    #[inline]
    pub fn alloc_cstring(&mut self, bytes: &[u8]) -> Option<*const i8> {
        let ptr = self.alloc(bytes.len() + 1, 1)?;

        unsafe {
            // Copy string bytes
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            // Null terminator
            *ptr.add(bytes.len()) = 0;
        }

        Some(ptr as *const i8)
    }

    /// Allocate a wide string (UTF-16) for Windows APIs.
    ///
    /// Converts UTF-8 to UTF-16LE and null-terminates.
    #[inline]
    #[allow(dead_code)]
    pub fn alloc_wstring(&mut self, s: &str) -> Option<*const u16> {
        let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        let size = wide.len() * 2;
        let ptr = self.alloc(size, 2)?;

        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr, size);
        }

        Some(ptr as *const u16)
    }

    /// Reset the arena for the next call.
    ///
    /// This is O(1) - just resets the offset to 0.
    #[inline]
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Get the current allocation offset (for debugging).
    #[inline]
    #[allow(dead_code)]
    pub fn used(&self) -> usize {
        self.offset
    }

    /// Get the high water mark (maximum usage so far).
    #[inline]
    #[allow(dead_code)]
    pub fn high_water_mark(&self) -> usize {
        self.high_water_mark
    }

    /// Get the total capacity.
    #[inline]
    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_cstring() {
        let mut arena = ScratchArena::new(1024);

        let ptr = arena.alloc_cstring(b"Hello").unwrap();
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(cstr.to_str().unwrap(), "Hello");

        assert_eq!(arena.used(), 6); // "Hello" + null
    }

    #[test]
    fn test_reset() {
        let mut arena = ScratchArena::new(1024);

        arena.alloc_cstring(b"Test1").unwrap();
        arena.alloc_cstring(b"Test2").unwrap();
        assert!(arena.used() > 0);

        arena.reset();
        assert_eq!(arena.used(), 0);
    }

    #[test]
    fn test_overflow() {
        let mut arena = ScratchArena::new(10);

        // Should succeed
        assert!(arena.alloc_cstring(b"Hi").is_some());

        // Should fail - not enough space
        assert!(arena.alloc_cstring(b"This is too long").is_none());
    }
}

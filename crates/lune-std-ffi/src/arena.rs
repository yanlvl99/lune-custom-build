//! Memory Arena for scoped allocations.
//!
//! Implements a bump allocator pattern with automatic cleanup.
//! All memory allocated through an arena is freed when the arena is dropped.
//!
//! # Safety
//! - Arena is !Send and !Sync - cannot be passed between threads
//! - Pointers become invalid when the arena is dropped

use mlua::prelude::*;
use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::cell::RefCell;

use crate::pointer::{RawPointer, next_arena_id};

/// A memory chunk allocated by the arena
struct Chunk {
    ptr: *mut u8,
    layout: Layout,
}

impl Drop for Chunk {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { dealloc(self.ptr, self.layout) };
        }
    }
}

/// A scoped memory arena (bump allocator)
///
/// All allocations are freed when the Arena is garbage collected.
/// The arena is NOT thread-safe (!Send, !Sync).
pub struct Arena {
    id: usize,
    chunks: RefCell<Vec<Chunk>>,
    total_allocated: RefCell<usize>,
    /// Marker to prevent Send/Sync
    _marker: std::marker::PhantomData<*mut ()>,
}

impl Arena {
    /// Create a new arena
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: next_arena_id(),
            chunks: RefCell::new(Vec::new()),
            total_allocated: RefCell::new(0),
            _marker: std::marker::PhantomData,
        }
    }

    /// Allocate memory from the arena
    pub fn alloc(&self, size: usize) -> LuaResult<RawPointer> {
        self.alloc_aligned(size, 8)
    }

    /// Allocate aligned memory from the arena
    pub fn alloc_aligned(&self, size: usize, align: usize) -> LuaResult<RawPointer> {
        if size == 0 {
            return Err(LuaError::external("Cannot allocate 0 bytes"));
        }

        let layout = Layout::from_size_align(size, align.max(1))
            .map_err(|e| LuaError::external(format!("Invalid layout: {}", e)))?;

        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err(LuaError::external("Allocation failed: out of memory"));
        }

        let chunk = Chunk { ptr, layout };
        self.chunks.borrow_mut().push(chunk);
        *self.total_allocated.borrow_mut() += size;

        Ok(RawPointer::managed(ptr.cast(), self.id, size))
    }

    /// Allocate memory for a specific type
    pub fn alloc_type(&self, ctype: crate::types::CType) -> LuaResult<RawPointer> {
        let size = ctype.size();
        let align = ctype.alignment();
        self.alloc_aligned(size, align)
    }

    /// Allocate an array of elements
    pub fn alloc_array(&self, ctype: crate::types::CType, count: usize) -> LuaResult<RawPointer> {
        if count == 0 {
            return Err(LuaError::external("Cannot allocate 0 elements"));
        }
        let size = ctype.size() * count;
        let align = ctype.alignment();
        self.alloc_aligned(size, align)
    }

    /// Get the arena ID
    #[must_use]
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get total bytes allocated
    #[must_use]
    pub fn total_allocated(&self) -> usize {
        *self.total_allocated.borrow()
    }

    /// Get number of allocations
    #[must_use]
    pub fn allocation_count(&self) -> usize {
        self.chunks.borrow().len()
    }

    /// Reset the arena, freeing all allocations
    pub fn reset(&self) {
        self.chunks.borrow_mut().clear();
        *self.total_allocated.borrow_mut() = 0;
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        // Chunks are automatically dropped via Vec's drop
        // Each Chunk's Drop impl deallocates its memory
    }
}

impl LuaUserData for Arena {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.id()));
        fields.add_field_method_get("totalAllocated", |_, this| Ok(this.total_allocated()));
        fields.add_field_method_get("allocationCount", |_, this| Ok(this.allocation_count()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // alloc(size) -> RawPointer
        methods.add_method("alloc", |_, this, size: usize| this.alloc(size));

        // allocAligned(size, align) -> RawPointer
        methods.add_method("allocAligned", |_, this, (size, align): (usize, usize)| {
            this.alloc_aligned(size, align)
        });

        // allocType(ctype) -> RawPointer
        methods.add_method("allocType", |_, this, ctype: crate::types::CType| {
            this.alloc_type(ctype)
        });

        // allocArray(ctype, count) -> RawPointer
        methods.add_method(
            "allocArray",
            |_, this, (ctype, count): (crate::types::CType, usize)| this.alloc_array(ctype, count),
        );

        // reset() - free all allocations
        methods.add_method("reset", |_, this, ()| {
            this.reset();
            Ok(())
        });

        // ToString
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "Arena(id={}, allocated={} bytes, chunks={})",
                this.id(),
                this.total_allocated(),
                this.allocation_count()
            ))
        });
    }
}

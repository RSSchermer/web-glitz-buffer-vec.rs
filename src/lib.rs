//! Implements growing [web-glitz](https://crates.io/crates/web-glitz) memory buffers for slices of
//! data. Buffers are automatically reallocated on update when the length of the new data exceeds
//! the current capacity of the buffer.
//!
//! For generic data, see [BufferVec]. For data that may be bound as vertex index data in draw
//! tasks, see [IndexBufferVec].

mod buffer_vec;
pub use self::buffer_vec::BufferVec;

mod index_buffer_vec;
pub use self::index_buffer_vec::IndexBufferVec;

mod util;

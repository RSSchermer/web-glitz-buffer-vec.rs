use std::borrow::Borrow;
use std::mem::MaybeUninit;

use web_glitz::buffer::UsageHint;
use web_glitz::pipeline::graphics::{IndexBuffer, IndexBufferView, IndexFormat};
use web_glitz::runtime::RenderingContext;

use crate::util::new_capacity_amortized;

/// A growable GPU buffer for data that may be used to specify vertex indices in a WebGlitz draw
/// task.
///
/// Elements must implement [IndexFormat].
///
/// # Example
/// ```
/// # use web_glitz::rendering::DefaultRGBBuffer;
/// # use web_glitz::rendering::DefaultRenderTarget;
/// # use web_glitz::buffer::BufferView;
/// # use web_glitz::pipeline::graphics::{GraphicsPipeline, Vertex};
/// # use web_glitz::runtime::RenderingContext;
/// # fn wrapper<Rc, V>(
/// #     context: Rc,
/// #     mut render_target: DefaultRenderTarget<DefaultRGBBuffer, ()>,
/// #     vertex_buffers: BufferView<[V]>,
/// #     graphics_pipeline: GraphicsPipeline<V, (), ()>
/// # )
/// # where
/// #     Rc: RenderingContext,
/// #     V: Vertex,
/// # {
/// # let resources = ();
/// use web_glitz_buffer_vec::IndexBufferVec;
/// use web_glitz::buffer::UsageHint;
///
/// let mut indices = IndexBufferVec::new(context, UsageHint::StaticDraw);
///
/// indices.update([1u16, 2u16, 3u16]);
///
/// let indices_view = indices.as_buffer_view();
///
/// assert_eq!(indices_view.len(), 3);
///
/// let render_pass = render_target.create_render_pass(|framebuffer| {
///     framebuffer.pipeline_task(&graphics_pipeline, |active_pipeline| {
///         active_pipeline.task_builder()
///             .bind_vertex_buffers(vertex_buffers)
///             .bind_index_buffer(indices_view)
///             .bind_resources(resources)
///             .draw_indexed(3, 1)
///             .finish()
///     })
/// });
/// # }
/// ```
///
/// Here `context` is a WebGlitz [RenderingContext]. For details on indexed rendering with WebGlitz,
/// see the [web_glitz::rendering] module documentation.
///
/// [IndexFormat]: web_glitz::pipeline::graphics::vertex::IndexFormat
/// [RenderingContext]: web_glitz::runtime::RenderingContext
pub struct IndexBufferVec<Rc, T> {
    context: Rc,
    len: usize,
    buffer: IndexBuffer<MaybeUninit<T>>,
}

impl<Rc, T> IndexBufferVec<Rc, T>
where
    Rc: RenderingContext,
    T: IndexFormat + 'static,
{
    /// Creates a new buffer-backed vector with 0 capacity for the given [RenderingContext].
    ///
    /// See [UsageHint] for details on GPU buffer performance hints.
    ///
    /// # Example
    ///
    /// ```
    /// # use web_glitz::runtime::RenderingContext;
    /// # fn wrapper<Rc>(context: Rc) where Rc: RenderingContext {
    /// use web_glitz_buffer_vec::IndexBufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut indices = IndexBufferVec::new(context, UsageHint::StaticDraw);
    ///
    /// assert_eq!(indices.capacity(), 0);
    /// # indices.update([1u16, 2u16, 3u16]);
    /// # }
    /// ```
    ///
    /// Here context is a [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    /// [UsageHint]: web_glitz::buffer::UsageHint
    pub fn new(context: Rc, usage: UsageHint) -> Self {
        let buffer = context.create_index_buffer_uninit(0, usage);

        IndexBufferVec {
            context,
            len: 0,
            buffer,
        }
    }

    /// Creates a new buffer-backed vector with the specified `capacity` for the given
    /// [RenderingContext].
    ///
    /// See [UsageHint] for details on GPU buffer performance hints.
    ///
    /// # Example
    ///
    /// ```
    /// # use web_glitz::runtime::RenderingContext;
    /// # fn wrapper<Rc>(context: Rc) where Rc: RenderingContext {
    /// use web_glitz_buffer_vec::IndexBufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut indices = IndexBufferVec::with_capacity(context, UsageHint::StaticDraw, 10);
    ///
    /// assert_eq!(indices.capacity(), 10);
    /// # indices.update([1u16, 2u16, 3u16]);
    /// # }
    /// ```
    ///
    /// Here context is a [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    /// [UsageHint]: web_glitz::buffer::UsageHint
    pub fn with_capacity(context: Rc, usage: UsageHint, capacity: usize) -> Self {
        let buffer = context.create_index_buffer_uninit(capacity, usage);

        IndexBufferVec {
            context,
            len: 0,
            buffer,
        }
    }

    /// Replaces the data in the buffer with the given `data`, resizing the buffer if necessary.
    ///
    /// # Guarantees
    ///
    /// Any task submitted from the same thread that called `update` after the update will see the
    /// new data. Any task that does not fence submitted from the same thread that called `update`
    /// before the update will see the old data. No other guarantees are given.
    ///
    /// # Example
    ///
    /// ```
    /// # use web_glitz::runtime::RenderingContext;
    /// # fn wrapper<Rc>(context: Rc) where Rc: RenderingContext {
    /// use web_glitz_buffer_vec::IndexBufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut indices = IndexBufferVec::new(context, UsageHint::StaticDraw);
    ///
    /// indices.update([1u16, 2u16, 3u16]);
    /// # }
    /// ```
    ///
    /// Here `context` is a WebGlitz [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    pub fn update<D>(&mut self, data: D)
    where
        D: Borrow<[T]> + Send + Sync + 'static,
    {
        let IndexBufferVec {
            context,
            len,
            buffer,
        } = self;

        *len = data.borrow().len();

        let current_capacity = buffer.len();

        if let Some(new_capacity) = new_capacity_amortized(current_capacity, *len) {
            *buffer = context
                .create_index_buffer_uninit(new_capacity, buffer.usage_hint())
                .into();
        }

        let view = buffer.get(0..*len).unwrap();

        let upload_task = unsafe {
            // Note: the view data range is not actually guaranteed to be initialized, but we're
            // only writing, not reading.
            view.assume_init().upload_command(data)
        };

        context.submit(upload_task);
    }

    /// The number of elements this vector can hold without allocating a new buffer.
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Returns a view on the data in the buffer.
    ///
    /// # Example
    ///
    /// ```
    /// # use web_glitz::runtime::RenderingContext;
    /// # fn wrapper<Rc>(context: Rc) where Rc: RenderingContext {
    /// use web_glitz_buffer_vec::IndexBufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut indices = IndexBufferVec::new(context, UsageHint::StaticDraw);
    ///
    /// indices.update([1u16, 2u16, 3u16]);
    ///
    /// let view = indices.as_buffer_view();
    ///
    /// assert_eq!(view.len(), 3);
    /// # }
    /// ```
    ///
    /// Here `context` is a WebGlitz [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    pub fn as_buffer_view(&self) -> IndexBufferView<T>
    where
        T: Copy + 'static,
    {
        let IndexBufferVec { len, buffer, .. } = self;

        unsafe { buffer.get(0..*len).unwrap().assume_init() }
    }
}

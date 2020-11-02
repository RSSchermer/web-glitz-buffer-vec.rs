use std::borrow::Borrow;
use std::mem::MaybeUninit;

use web_glitz::buffer::{Buffer, BufferView, UsageHint};
use web_glitz::runtime::RenderingContext;

use crate::util::new_capacity_amortized;

/// A growable GPU buffer for data that may be used to store GPU accessiable data that may be used
/// in WebGlitz tasks.
///
/// Elements must implement [Copy].
///
/// # Example
/// ```
/// # #![feature(const_fn, const_maybe_uninit_as_ptr, const_ptr_offset_from, const_raw_ptr_deref, ptr_offset_from)]
/// # use web_glitz::rendering::DefaultRGBBuffer;
/// # use web_glitz::rendering::DefaultRenderTarget;
/// # use web_glitz::buffer::BufferView;
/// # use web_glitz::pipeline::graphics::GraphicsPipeline;
/// # use web_glitz::runtime::RenderingContext;
/// use web_glitz_buffer_vec::BufferVec;
/// use web_glitz::buffer::UsageHint;
///
/// #[derive(web_glitz::derive::Vertex, Clone, Copy)]
/// struct Vertex {
///     #[vertex_attribute(location = 0, format = "Float2_f32")]
///     position: [f32; 2],
/// }
///
/// # fn wrapper<Rc>(
/// #     context: Rc,
/// #     mut render_target: DefaultRenderTarget<DefaultRGBBuffer, ()>,
/// #     graphics_pipeline: GraphicsPipeline<Vertex, (), ()>
/// # )
/// # where
/// #     Rc: RenderingContext,
/// # {
/// # let resources = ();
/// let mut vertices = BufferVec::new(context, UsageHint::StaticDraw);
///
/// vertices.update([
///     Vertex { position: [-0.5, -0.5] },
///     Vertex { position: [0.5, -0.5] },
///     Vertex { position: [0.0, 0.5] },
/// ]);
///
/// let vertices_view = vertices.as_buffer_view();
///
/// assert_eq!(vertices_view.len(), 3);
///
/// let render_pass = render_target.create_render_pass(|framebuffer| {
///     framebuffer.pipeline_task(&graphics_pipeline, |active_pipeline| {
///         active_pipeline.task_builder()
///             .bind_vertex_buffers(vertices_view)
///             .bind_resources(resources)
///             .draw(3, 1)
///             .finish()
///     })
/// });
/// # }
/// ```
///
/// Here `context` is a WebGlitz [RenderingContext]. For details on rendering with WebGlitz, see the
/// [web_glitz::rendering] module documentation.
///
/// [RenderingContext]: web_glitz::runtime::RenderingContext
pub struct BufferVec<Rc, T> {
    context: Rc,
    len: usize,
    buffer: Buffer<[MaybeUninit<T>]>,
}

impl<Rc, T> BufferVec<Rc, T>
where
    Rc: RenderingContext,
    T: Copy + 'static,
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
    /// use web_glitz_buffer_vec::BufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut vec = BufferVec::new(context, UsageHint::StaticDraw);
    ///
    /// assert_eq!(vec.capacity(), 0);
    /// # vec.update([1, 2, 3]);
    /// # }
    /// ```
    ///
    /// Here context is a [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    /// [UsageHint]: web_glitz::buffer::UsageHint
    pub fn new(context: Rc, usage: UsageHint) -> Self {
        let buffer = context.create_buffer_slice_uninit(0, usage);

        BufferVec {
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
    /// use web_glitz_buffer_vec::BufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut vec = BufferVec::with_capacity(context, UsageHint::StaticDraw, 10);
    ///
    /// assert_eq!(vec.capacity(), 10);
    /// # vec.update([1, 2, 3]);
    /// # }
    /// ```
    ///
    /// Here context is a [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    /// [UsageHint]: web_glitz::buffer::UsageHint
    pub fn with_capacity(context: Rc, usage: UsageHint, capacity: usize) -> Self {
        let buffer = context.create_buffer_slice_uninit(capacity, usage);

        BufferVec {
            context,
            len: 0,
            buffer,
        }
    }

    /// Replaces the data in the buffer with the given `data`, resizing the buffer if necessary.
    ///
    /// Returns `true` if a new buffer was allocated, `false` otherwise.
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
    /// use web_glitz_buffer_vec::BufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut vec = BufferVec::new(context, UsageHint::StaticDraw);
    ///
    /// vec.update([1, 2, 3]);
    /// # }
    /// ```
    ///
    /// Here `context` is a WebGlitz [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    pub fn update<D>(&mut self, data: D) -> bool
    where
        D: Borrow<[T]> + Send + Sync + 'static,
    {
        let BufferVec {
            context,
            len,
            buffer,
        } = self;

        *len = data.borrow().len();

        let current_capacity = buffer.len();

        let reallocated = if let Some(new_capacity) = new_capacity_amortized(current_capacity, *len) {
            *buffer = context
                .create_buffer_slice_uninit(new_capacity, buffer.usage_hint())
                .into();

            true
        } else {
            false
        };

        let view = buffer.get(0..*len).unwrap();

        let upload_task = unsafe {
            // Note: the view data range is not actually guaranteed to be initialized, but we're
            // only writing, not reading.
            view.assume_init().upload_command(data)
        };

        context.submit(upload_task);

        reallocated
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
    /// use web_glitz_buffer_vec::BufferVec;
    /// use web_glitz::buffer::UsageHint;
    ///
    /// let mut vec = BufferVec::new(context, UsageHint::StaticDraw);
    ///
    /// vec.update([1, 2, 3]);
    ///
    /// let view = vec.as_buffer_view();
    ///
    /// assert_eq!(view.len(), 3);
    /// # }
    /// ```
    ///
    /// Here `context` is a WebGlitz [RenderingContext].
    ///
    /// [RenderingContext]: web_glitz::runtime::RenderingContext
    pub fn as_buffer_view(&self) -> BufferView<[T]>
    where
        T: Copy + 'static,
    {
        let BufferVec { len, buffer, .. } = self;

        unsafe { buffer.get(0..*len).unwrap().assume_init() }
    }
}

use std::{fmt::Debug, marker::PhantomData, ops::Range};

use bytemuck::Pod;

pub const QUAD_VERT_ORDER: [u16; 6] = [0, 1, 2, 1, 2, 3];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mesh<V, Index = u16> {
    pub vertices: Vec<V>,
    pub indices: Vec<Index>,
}

impl<V, Index> Mesh<V, Index> {
    pub fn new() -> Self
    where
        V: Default,
        Index: Default,
    {
        Self::default()
    }
}

impl<V> Mesh<V, u32> {
    pub fn push_vertices(
        &mut self,
        vertices: impl IntoIterator<Item = V>,
        indices: impl IntoIterator<Item = u32>,
    ) {
        self.vertices.extend(vertices);

        self.indices
            .extend(indices.into_iter().map(|i| i + self.vertices.len() as u32));
    }
}

impl<V> Mesh<V, u16> {
    pub fn push_vertices(
        &mut self,
        vertices: impl IntoIterator<Item = V>,
        indices: impl IntoIterator<Item = u16>,
    ) {
        self.vertices.extend(vertices);

        self.indices
            .extend(indices.into_iter().map(|i| i + self.vertices.len() as u16));
    }
}

pub struct DynamicGPUBuffer<T: Sized + Pod> {
    pub buffer: wgpu::Buffer,
    size: u64,
    pub usage: wgpu::BufferUsages,
    _data: PhantomData<T>,
}

impl<T: Sized + Pod> DynamicGPUBuffer<T> {
    pub fn new(device: &wgpu::Device, initial_cap_count: u64, usage: wgpu::BufferUsages) -> Self {
        let initial_size = initial_cap_count * (std::mem::size_of::<T>() as u64);
        let buffer = Self::create_buffer(device, initial_size, usage);

        Self {
            buffer,
            usage,
            size: 0,
            _data: PhantomData,
        }
    }

    fn create_buffer(
        device: &wgpu::Device,
        desired_size: u64,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        let next_cap = desired_size.next_power_of_two();

        device.create_buffer(&wgpu::BufferDescriptor {
            label: "dynamic gpu buffer".into(),
            size: next_cap,
            usage,
            mapped_at_creation: false,
        })
    }

    fn reallocate_self(&mut self, device: &wgpu::Device, size: u64) -> bool {
        if size > self.buffer.size() {
            self.buffer = Self::create_buffer(device, size, self.usage);
            true
        } else {
            false
        }
    }

    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, items: &[T], num_items: u64) -> bool {
        debug_assert!(num_items <= items.len() as u64);

        let new_size = num_items * (std::mem::size_of::<T>() as u64);

        let reallocated = self.reallocate_self(device, new_size);

        self.size = new_size;
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(items));

        reallocated
    }

    pub const fn count(&self) -> u64 {
        self.size / (std::mem::size_of::<T>() as u64)
    }

    pub const fn size(&self) -> u64 {
        self.size
    }
}

pub struct DynamicGPUMeshTriBuffer<T: Sized + Pod + Debug> {
    vertex_buffer: DynamicGPUBuffer<T>,
    index_buffer: DynamicGPUBuffer<u16>,

    _data: PhantomData<Vec<T>>,
}

impl<T: Sized + Pod + Debug> DynamicGPUMeshTriBuffer<T> {
    const MIN_CAP_TRIS: u64 = 16;
    const MIN_CAP_VERTICES: u64 = 64;

    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = DynamicGPUBuffer::new(
            device,
            Self::MIN_CAP_TRIS,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );

        let index_buffer = DynamicGPUBuffer::new(
            device,
            Self::MIN_CAP_VERTICES,
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        );

        Self {
            vertex_buffer,
            index_buffer,

            _data: PhantomData,
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: Range<u32>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.buffer.slice(..));

        render_pass.set_index_buffer(
            self.index_buffer.buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        render_pass.draw_indexed(0..self.index_buffer.count() as u32, 0, instances);
    }

    pub fn write_all(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        mut buffers: VertexBuffers<T>,
    ) {
        self.vertex_buffer.write(device, queue, &buffers.vertices, buffers.vertices.len() as u64);

        // have to do padding to be 4-byte aligned
        // TODO: use wgpu::COPY_BUFFER_ALIGNMENT instead of "4"

        let num_indices = buffers.indices.len();
        for _ in 0..4 - (buffers.indices.len() % 4) {
            buffers.indices.push(0u16);
        }

        self.index_buffer.write(device, queue, &buffers.indices, num_indices as u64);
    }
}

pub trait PushVertices<T> {
    fn push_vertices(
        &mut self,
        vertices: impl IntoIterator<Item = T>,
        indices: impl IntoIterator<Item = u16>,
    );

    fn push_quads(&mut self, quads: impl IntoIterator<Item = [T; 4]>) {
        for quad in quads {
            self.push_vertices(quad, QUAD_VERT_ORDER);
        }
    }
}

impl<V> PushVertices<V> for VertexBuffers<V> {
    fn push_vertices(
        &mut self,
        vertices: impl IntoIterator<Item = V>,
        indices: impl IntoIterator<Item = u16>,
    ) {
        let index_offset = self.vertices.len() as u16;

        self.vertices.extend(vertices.into_iter());

        self.indices
            .extend(indices.into_iter().map(|idx| idx + index_offset));
    }
}

pub type VertexBuffers<V> = lyon::tessellation::VertexBuffers<V, u16>;

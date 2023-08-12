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

pub struct DynamicGPUMeshTriBuffer<T: Sized + Pod + Debug> {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    cap_indices: u64,
    num_indices: u64,

    vb_cap: u64,
    vb_count: u64,

    _data: PhantomData<Vec<T>>,
}

impl<T: Sized + Pod + Debug> DynamicGPUMeshTriBuffer<T> {
    const MIN_CAP_TRIS: u64 = 16;
    const MIN_CAP_VERTICES: u64 = 64;

    const VERTEX_BYTES: u64 = std::mem::size_of::<T>() as u64;
    const INDEX_BYTES: u64 = std::mem::size_of::<u16>() as u64;

    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = Self::create_vertex_buffer(Self::MIN_CAP_VERTICES, device);
        let index_buffer = Self::create_index_buffer(Self::MIN_CAP_TRIS, device);

        Self {
            vertex_buffer,
            index_buffer,

            cap_indices: Self::MIN_CAP_TRIS,
            num_indices: 0,

            vb_cap: Self::MIN_CAP_VERTICES,
            vb_count: 0,

            _data: PhantomData,
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: Range<u32>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.draw_indexed(0..self.num_indices as u32, 0, instances);
    }

    pub fn write_all(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        buffers: &VertexBuffers<T>,
    ) {
        self.resize_buffers(
            device,
            Self::index_buffer_size(buffers.indices.len() as u64),
            Self::vertex_buffer_size(buffers.vertices.len() as u64),
        );

        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&buffers.vertices),
        );

        queue.write_buffer(
            &self.index_buffer,
            0,
            bytemuck::cast_slice(&buffers.indices),
        );
    }

    fn resize_buffers(&mut self, device: &wgpu::Device, num_indices: u64, num_vertices: u64) {
        self.vb_count = num_vertices;
        self.num_indices = num_indices;

        self.reallocate_buffers(device);
    }

    fn vertex_buffer_size(count: u64) -> u64 {
        count * Self::VERTEX_BYTES
    }

    fn index_buffer_size(count: u64) -> u64 {
        count * Self::INDEX_BYTES
    }

    fn reallocate_buffers(&mut self, device: &wgpu::Device) {
        if self.vb_count > self.vb_cap {
            let next_cap = self.vb_count.next_power_of_two();
            self.vertex_buffer = Self::create_vertex_buffer(next_cap, device);
            self.vb_cap = next_cap;
        }

        if self.num_indices > self.cap_indices {
            let next_cap = self.num_indices.next_power_of_two();
            self.index_buffer = Self::create_index_buffer(next_cap, device);
            self.cap_indices = next_cap;
        }
    }

    fn create_vertex_buffer(count: u64, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tri buffer allocator vertex buffer"),
            size: Self::vertex_buffer_size(count),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_index_buffer(count: u64, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tri buffer allocator index buffer"),
            size: Self::index_buffer_size(count),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
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

use std::{fmt::Debug, marker::PhantomData, ops::Range, sync::Mutex};

use bytemuck::Pod;

use crate::mesh::Mesh;

pub struct DynamicGPUQuadBuffer<T: Sized + Pod + Debug> {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    cap_quads: u64,
    num_quads: u64,

    draw_idx: Mutex<u64>,

    _data: PhantomData<Vec<T>>,
}

impl<T: Sized + Pod + Debug> DynamicGPUQuadBuffer<T> {
    const MIN_CAP_QUADS: u64 = 16;

    const QUAD_VERTEX_BYTES: u64 = std::mem::size_of::<T>() as u64 * 4;
    const QUAD_INDEX_BYTES: u64 = 6 * 2;

    pub fn new(device: &wgpu::Device) -> Self {
        let (vertex_buffer, index_buffer) = Self::create_buffers(device, Self::MIN_CAP_QUADS);

        Self {
            vertex_buffer,
            index_buffer,
            cap_quads: Self::MIN_CAP_QUADS,
            num_quads: Default::default(),
            draw_idx: Mutex::new(Default::default()),
            _data: PhantomData,
        }
    }

    pub fn vertex(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn render_quads<'a>(
        &'a self,
        render_pipeline: &'a wgpu::RenderPipeline,
        bind_group: &'a wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'a>,
        quads: u64,
        instances: Range<u32>,
    ) {
        let mut draw_idx_mut = self.draw_idx.lock().unwrap();
        let draw_idx = *draw_idx_mut;

        *draw_idx_mut += quads;

        self.render_quad_range(
            render_pipeline,
            bind_group,
            render_pass,
            draw_idx..draw_idx + quads,
            instances,
        );
    }

    fn render_quad_range<'a>(
        &'a self,
        render_pipeline: &'a wgpu::RenderPipeline,
        bind_group: &'a wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'a>,
        quads: Range<u64>,
        instances: Range<u32>,
    ) {
        if self.num_quads == 0 {
            return;
        }

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        render_pass.set_index_buffer(
            self.index_buffer.slice(Range {
                start: quads.start * Self::QUAD_INDEX_BYTES,
                end: quads.end * Self::QUAD_INDEX_BYTES,
            }),
            wgpu::IndexFormat::Uint16,
        );

        let num_quads = (quads.end - quads.start) as u32;

        render_pass.draw_indexed(0..num_quads * 6, 0, instances);
    }

    pub fn write_all_quads(&self, queue: &wgpu::Queue, quads: impl Iterator<Item = [T; 4]>) {
        let (vertex_size, index_size) = Self::buffer_sizes(self.num_quads);

        let mut vertex_buffer_bytes = Vec::<u8>::with_capacity(vertex_size as usize);
        let mut index_buffer_bytes = Vec::<u8>::with_capacity(index_size as usize);

        let mut index_window = [0u16, 1, 2, 1, 2, 3];

        for quad in quads {
            vertex_buffer_bytes.extend_from_slice(bytemuck::bytes_of(&quad));
            index_buffer_bytes.extend_from_slice(bytemuck::bytes_of(&index_window));

            for idx in index_window.iter_mut() {
                *idx += 4;
            }
        }

        queue.write_buffer(&self.vertex_buffer, 0, vertex_buffer_bytes.as_slice());
        queue.write_buffer(&self.index_buffer, 0, index_buffer_bytes.as_slice());
    }

    const fn buffer_sizes(count: u64) -> (u64, u64) {
        (
            Self::QUAD_VERTEX_BYTES * count,
            Self::QUAD_INDEX_BYTES * count,
        )
    }

    pub fn set_num_quads(&mut self, device: &wgpu::Device, num_quads: u64) {
        self.num_quads = num_quads;
        self.reallocate_buffers(device);

        *self.draw_idx.lock().unwrap() = 0;
    }

    fn reallocate_buffers(&mut self, device: &wgpu::Device) {
        if self.num_quads <= self.cap_quads {
            return;
        }

        let next_cap = self.num_quads.next_power_of_two();

        let (vertex_buffer, index_buffer) = Self::create_buffers(device, next_cap);

        self.vertex_buffer = vertex_buffer;
        self.index_buffer = index_buffer;

        self.cap_quads = next_cap;
    }

    fn create_buffers(device: &wgpu::Device, count: u64) -> (wgpu::Buffer, wgpu::Buffer) {
        let (vertex_size, index_size) = Self::buffer_sizes(count);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("buffer allocator vertex buffer"),
            size: vertex_size.next_power_of_two(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("buffer allocator index buffer"),
            size: index_size.next_power_of_two(),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        return (vertex_buffer, index_buffer);
    }

    pub fn num_quads(&self) -> u64 {
        self.num_quads
    }
}

pub struct DynamicGPUMeshTriBuffer<T: Sized + Pod + Debug> {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    cap_indices: u64,
    num_indices: u64,

    vb_cap: u64,
    vb_count: u64,

    draw_idx: Mutex<u64>,

    _data: PhantomData<Vec<T>>,
}

impl<T: Sized + Pod + Debug> DynamicGPUMeshTriBuffer<T> {
    const MIN_CAP_TRIS: u64 = 16;
    const MIN_CAP_VERTICES: u64 = 64;

    const VERTEX_BYTES: u64 = std::mem::size_of::<T>() as u64;
    const INDEX_BYTES: u64 = 2;

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

            draw_idx: Mutex::new(Default::default()),

            _data: PhantomData,
        }
    }

    pub fn render_indices<'a>(
        &'a self,
        render_pipeline: &'a wgpu::RenderPipeline,
        bind_group: &'a wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'a>,
        indices: u64,
        instances: Range<u32>,
    ) {
        let mut draw_idx_mut = self.draw_idx.lock().unwrap();
        let draw_idx = *draw_idx_mut;

        *draw_idx_mut += indices;

        let range: Range<u64> = draw_idx..draw_idx + indices;

        if self.num_indices == 0 {
            return;
        }

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.draw_indexed(range.start as u32..range.end as u32, 0, instances);
    }

    pub fn write_all_meshes(&mut self, queue: &wgpu::Queue, meshes: impl Iterator<Item = Mesh<T>>) {
        let vertex_size = Self::vertex_buffer_size(self.vb_count);
        let index_size = Self::index_buffer_size(self.num_indices);

        let mut vertex_buffer_bytes = Vec::<u8>::with_capacity(vertex_size as usize);
        let mut index_buffer_bytes = Vec::<u8>::with_capacity(index_size as usize);

        let mut offset = 0;

        for Mesh {
            mut indices,
            vertices,
        } in meshes
        {
            if offset > 0 {
                for i in indices.iter_mut() {
                    *i += offset as u16;
                }
            }

            index_buffer_bytes.extend_from_slice(bytemuck::cast_slice(&indices));
            vertex_buffer_bytes.extend_from_slice(bytemuck::cast_slice(&vertices));

            offset += vertices.len();
        }

        while vertex_buffer_bytes.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT != 0 {
            vertex_buffer_bytes.push(0)
        }

        while index_buffer_bytes.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT != 0 {
            index_buffer_bytes.push(0)
        }

        queue.write_buffer(&self.vertex_buffer, 0, vertex_buffer_bytes.as_slice());
        queue.write_buffer(&self.index_buffer, 0, index_buffer_bytes.as_slice());
    }

    pub fn set_num_verts(&mut self, device: &wgpu::Device, num_indices: u64, num_vertices: u64) {
        self.vb_count = num_vertices;
        self.num_indices = num_indices;

        self.reallocate_buffers(device);

        *self.draw_idx.lock().unwrap() = 0;
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

pub type VertexBuffers<V> = lyon::tessellation::VertexBuffers<V, u16>;

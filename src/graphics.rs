use std::{marker::PhantomData, ops::Range};

use bytemuck::Pod;
use wgpu::RenderPass;

use crate::util::{MapRange, ScaleRange};

pub struct DynamicGPUQuadBuffer<T: Sized + Pod> {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    cap_quads: u64,
    num_quads: u64,

    draw_idx: u64,

    _data: PhantomData<Vec<T>>,
}

impl<T: Sized + Pod> DynamicGPUQuadBuffer<T> {
    const MIN_CAP_QUADS: u64 = 16;

    const QUAD_VERTEX_BYTES: u64 = std::mem::size_of::<T>() as u64 * 4;
    const QUAD_INDEX_BYTES: u64 = 6 * 2;

    pub fn new(device: &wgpu::Device) -> Self {
        let (vertex_buffer, index_buffer) = Self::create_buffers(device, Self::MIN_CAP_QUADS);

        Self {
            vertex_buffer,
            index_buffer,
            cap_quads: Self::MIN_CAP_QUADS,
            num_quads: 0,
            draw_idx: 0,
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
        &'a mut self,
        render_pipeline: &'a wgpu::RenderPipeline,
        bind_group: &'a wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'a>,
        quads: u64,
        instances: Range<u32>,
    ) {
        let draw_idx = self.draw_idx;
        self.draw_idx += quads;

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

        render_pass.set_vertex_buffer(
            0,
            self.vertex_buffer
                .slice(quads.scale(Self::QUAD_VERTEX_BYTES)),
        );

        render_pass.set_index_buffer(
            self.index_buffer.slice(quads.scale(Self::QUAD_INDEX_BYTES)),
            wgpu::IndexFormat::Uint16,
        );

        render_pass.draw_indexed(quads.map_range(|x| x as u32).scale(6), 0, instances);
    }

    // pub fn render_all_quads<'a>(
    //     &'a self,
    //     render_pipeline: &'a wgpu::RenderPipeline,
    //     bind_group: &'a wgpu::BindGroup,
    //     render_pass: &mut wgpu::RenderPass<'a>,
    //     instances: Range<u32>,
    // ) {
    //     self.render_quads(
    //         render_pipeline,
    //         bind_group,
    //         render_pass,
    //         0..self.num_quads,
    //         instances,
    //     );
    // }

    // fn draw_all_quads<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: Range<u32>) {
    //     render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    //     render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

    //     render_pass.draw_indexed(0..(self.num_quads * 6) as u32, 0, instances);
    // }

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

        self.draw_idx = 0;
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

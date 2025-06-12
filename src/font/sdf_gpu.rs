use crate::svg::SvgInfo;
use pi_share::Share;
use pi_wgpu::{self as wgpu, util::DeviceExt};

pub fn create_indices() -> [u16; 6] {
    [0, 1, 2, 1, 2, 3]
}

pub struct GPUState {
    device: Share<wgpu::Device>,
    queue: Share<wgpu::Queue>,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout0: wgpu::BindGroupLayout,
    bind_group_layout1: wgpu::BindGroupLayout,
}

impl GPUState {
    pub fn init(device: Share<wgpu::Device>, queue: Share<wgpu::Queue>) -> Self {
        let vs: pi_wgpu::ShaderModule = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Glsl {
                shader: include_str!("../../shader/glyphy.vs").into(),
                stage: naga::ShaderStage::Vertex,
                defines: Default::default(),
            },
        });

        // Load the shaders from disk
        let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Glsl {
                shader: include_str!("../../shader/glyphy.fs").into(),
                stage: naga::ShaderStage::Fragment,
                defines: Default::default(),
            },
        });

        let bind_group_layout0 =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group_layout1 =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(16),
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout0, &bind_group_layout1],
            push_constant_ranges: &[],
        });

        let primitive = wgpu::PrimitiveState::default();
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs,
                entry_point: Some("main"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 1,
                        }],
                    },
                ],
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[],
                    zero_initialize_workgroup_memory: true,
                },
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState::from(
                    wgpu::TextureFormat::R8Unorm,
                ))],
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[],
                    zero_initialize_workgroup_memory: true,
                },
            }),
            primitive,
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            bind_group_layout0,
            bind_group_layout1,
            device,
            queue,
        }
    }

    pub fn draw(
        &self,
        texture: &wgpu::Texture,
        info: SvgInfo,
        tex_offset: (u32, u32),
        tex_size: u32,
        pxrange: f32,
        cur_off: u32,
        scale: f32,
    ) {
        let sdf_tex = info.compute_sdf_cell(scale);

        let size = tex_size as f32 * (scale + 1.0); //
        let uv = (1.0 - (tex_size as f32 + cur_off as f32 * 2.0) / size) * 0.5;

        let verties = [
            -1.0f32,
            -1.0,
            uv,
            uv,
            -1.0,
            1.0,
            uv,
            1.0 - uv,
            1.0,
            -1.0,
            1.0 - uv,
            uv,
            1.0,
            1.0,
            1.0 - uv,
            1.0 - uv,
        ]; // 获取网格数据

        let index_tex_size = (
            sdf_tex.tex_info.grid_w as u32,
            sdf_tex.tex_info.grid_h as u32,
        );
        // 创建索引纹理
        let index_tex = &sdf_tex.index_tex;
        let index_texture_extent = wgpu::Extent3d {
            width: index_tex_size.0 as u32,
            height: index_tex_size.1 as u32,
            depth_or_array_layers: 1,
        };

        let index_tex_sampler = self
            .device
            .create_sampler(&wgpu::SamplerDescriptor::default());
        let index_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: index_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let index_texture_view = index_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.queue.write_texture(
            index_texture.as_image_copy(),
            index_tex,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(index_tex_size.0 as u32 * 2),
                rows_per_image: None,
            },
            index_texture_extent,
        );

        let bind_group0 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout0,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&index_tex_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&index_texture_view),
                },
            ],
            label: None,
        });

        let data_tex = &sdf_tex.data_tex;
        let data_texture_extent = wgpu::Extent3d {
            width: sdf_tex.data_tex.len() as u32 / 4,
            height: 1,
            depth_or_array_layers: 1,
        };

        let data_tex_sampler = self
            .device
            .create_sampler(&wgpu::SamplerDescriptor::default());
        let data_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: data_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        println!("=========== pxrange: {}", pxrange);
        let data_tex_size_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("u_weight_and_offset_buffer"),
                    contents: bytemuck::cast_slice(&[
                        (sdf_tex.data_tex.len() / 4) as f32,
                        pxrange as f32,
                        if info.is_area { 1.0 } else { 0.0 }, 0.0
                    ]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let data_texture_view = data_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.queue.write_texture(
            data_texture.as_image_copy(),
            data_tex,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(data_tex.len() as u32),
                rows_per_image: None,
            },
            data_texture_extent,
        );

        let bind_group1 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout1,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&data_tex_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&data_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &data_tex_size_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(8),
                    }),
                },
            ],
            label: None,
        });

        // 创建网格数据
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&verties),
                usage: wgpu::BufferUsages::VERTEX,
            });

        // 以下为实例化数据
        let u_info = vec![
            sdf_tex.tex_info.max_offset as f32,
            sdf_tex.tex_info.min_sdf as f32,
            sdf_tex.tex_info.sdf_step as f32,
            sdf_tex.tex_info.cell_size * 0.5 * 2.0f32.sqrt(),
        ];

        let u_info_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("u_info_buffer"),
                contents: bytemuck::cast_slice(&u_info),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_data = create_indices();
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&index_data),
                usage: wgpu::BufferUsages::INDEX,
            });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // rpass.push_debug_group("Prepare data for draw.");
            let size = tex_size + cur_off * 2;
            println!(
                "========== set_viewport: {:?}",
                (
                    tex_offset.0 as f32,
                    tex_offset.1 as f32,
                    size as f32,
                    size as f32
                )
            );
            rpass.set_viewport(
                tex_offset.0 as f32,
                tex_offset.1 as f32,
                size as f32,
                size as f32,
                0.0,
                1.0,
            );
            rpass.set_pipeline(&self.render_pipeline);

            rpass.set_bind_group(0, &bind_group0, &[]);
            rpass.set_bind_group(1, &bind_group1, &[]);

            rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, u_info_buffer.slice(..));

            rpass.draw_indexed(0..6, 0, 0..1 as u32);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}

use crate::{GpuData, Viewport};
use glutin::dpi::LogicalSize;
use glutin::ElementState;
use std::error::Error;

struct FrameTime(f32);

impl FrameTime {
    pub fn update(&mut self, t: f32) {
        self.0 = self.0 * 0.95 + 0.05 * t;
    }
}

pub unsafe fn run(name: &'static str, gpu_data: GpuData) -> Result<(), Box<dyn Error>> {
    let mut events_loop = glutin::EventsLoop::new();
    let wb = glutin::WindowBuilder::new()
        .with_title(name)
        .with_dimensions(LogicalSize {
            width: 1240.0,
            height: 700.0,
        });
    let window = glutin::ContextBuilder::new()
        .with_vsync(false)
        .with_srgb(true)
        .with_gl_debug_flag(true)
        .build_windowed(wb, &events_loop)?
        .make_current()
        .unwrap();

    let LogicalSize {
        width: w,
        height: h,
    } = window.window().get_inner_size().unwrap();

    let grr = grr::Device::new(
        |symbol| window.get_proc_address(symbol) as *const _,
        grr::Debug::Enable {
            callback: |report, _, _, _, msg| {
                println!("{:?}: {:?}", report, msg);
            },
            flags: grr::DebugReport::FULL,
        },
    );
    let vertex_array = grr.create_vertex_array(&[
        grr::VertexAttributeDesc {
            location: 0,
            binding: 0,
            format: grr::VertexFormat::Xy32Float, // pos world
            offset: 0,
        },
        grr::VertexAttributeDesc {
            location: 1,
            binding: 0,
            format: grr::VertexFormat::Xy32Float, // pos curve
            offset: 2 * std::mem::size_of::<f32>() as u32,
        },
        grr::VertexAttributeDesc {
            location: 2,
            binding: 1,
            format: grr::VertexFormat::Xyz32Uint, // curve range
            offset: 0,
        },
    ])?;

    let color_blend = grr::ColorBlend {
        attachments: vec![grr::ColorBlendAttachment {
            blend_enable: true,
            color: grr::BlendChannel {
                src_factor: grr::BlendFactor::SrcAlpha,
                dst_factor: grr::BlendFactor::OneMinusSrcAlpha,
                blend_op: grr::BlendOp::Add,
            },
            alpha: grr::BlendChannel {
                src_factor: grr::BlendFactor::SrcAlpha,
                dst_factor: grr::BlendFactor::OneMinusSrcAlpha,
                blend_op: grr::BlendOp::Add,
            },
        }],
    };

    let shader_vs = grr.create_shader(
        grr::ShaderStage::Vertex,
        include_bytes!("../assets/lanka.vs"),
    )?;
    let shader_fs = grr.create_shader(
        grr::ShaderStage::Fragment,
        include_bytes!("../assets/lanka.fs"),
    )?;
    let pipeline_raster = grr.create_graphics_pipeline(grr::VertexPipelineDesc {
        vertex_shader: shader_vs,
        geometry_shader: None,
        tessellation_evaluation_shader: None,
        tessellation_control_shader: None,
        fragment_shader: Some(shader_fs),
    })?;

    let gpu_vertices = grr.create_buffer_from_host(
        grr::as_u8_slice(&gpu_data.vertices),
        grr::MemoryFlags::empty(),
    )?;
    let gpu_bbox =
        grr.create_buffer_from_host(grr::as_u8_slice(&gpu_data.bbox), grr::MemoryFlags::empty())?;
    let gpu_primitives = grr.create_buffer_from_host(
        grr::as_u8_slice(&gpu_data.primitives),
        grr::MemoryFlags::empty(),
    )?;
    let gpu_curve_ranges = grr.create_buffer_from_host(
        grr::as_u8_slice(&gpu_data.curve_ranges),
        grr::MemoryFlags::empty(),
    )?;

    let mut running = true;

    let mut time_last = std::time::Instant::now();
    let mut avg_frametime_cpu = FrameTime(0.0);
    let mut avg_frametime_gpu = FrameTime(0.0);

    let mut viewport = Viewport {
        position: (0.0, 0.0),
        scaling_y: h as _,
        aspect_ratio: (w / h) as _,
    };

    let query = [
        grr.create_query(grr::QueryType::Timestamp),
        grr.create_query(grr::QueryType::Timestamp),
    ];

    let mut mouse1 = ElementState::Released;

    while running {
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => running = false,
                glutin::WindowEvent::Resized(size) => {
                    let dpi_factor = window.window().get_hidpi_factor();
                    window.resize(size.to_physical(dpi_factor));
                }
                _ => (),
            },
            glutin::Event::DeviceEvent {
                event: glutin::DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if mouse1 == ElementState::Pressed {
                    let scale = viewport.get_scale();
                    viewport.position.0 -= scale.0 * (delta.0 / w) as f32;
                    viewport.position.1 += scale.1 * (delta.1 / h) as f32;
                }
            }
            glutin::Event::DeviceEvent {
                event:
                    glutin::DeviceEvent::MouseWheel {
                        delta: glutin::MouseScrollDelta::LineDelta(_, delta),
                    },
                ..
            } => {
                viewport.scaling_y *= (delta * -0.1).exp();
            }
            glutin::Event::DeviceEvent {
                event: glutin::DeviceEvent::Button { state, .. },
                ..
            } => {
                mouse1 = state;
            }
            _ => (),
        });

        // timing
        let time_now = std::time::Instant::now();
        let elapsed = time_now.duration_since(time_last).as_micros() as f32 / 1_000_000.0;
        time_last = time_now;
        avg_frametime_cpu.update(elapsed);
        window.window().set_title(&format!(
            "grr-2d :: frame: cpu: {:.2} ms | gpu: {:.2} ms",
            avg_frametime_cpu.0 * 1000.0,
            avg_frametime_gpu.0 * 1000.0,
        ));

        grr.bind_vertex_array(vertex_array);
        grr.bind_vertex_buffers(
            vertex_array,
            0,
            &[
                grr::VertexBufferView {
                    buffer: gpu_bbox,
                    offset: 0,
                    stride: (std::mem::size_of::<f32>() * 4) as _,
                    input_rate: grr::InputRate::Vertex,
                },
                grr::VertexBufferView {
                    buffer: gpu_curve_ranges,
                    offset: 0,
                    stride: (std::mem::size_of::<u32>() * 3) as _,
                    input_rate: grr::InputRate::Vertex,
                },
            ],
        );

        grr.set_viewport(
            0,
            &[grr::Viewport {
                x: 0.0,
                y: 0.0,
                w: w as _,
                h: h as _,
                n: 0.0,
                f: 1.0,
            }],
        );
        grr.set_scissor(
            0,
            &[grr::Region {
                x: 0,
                y: 0,
                w: w as _,
                h: h as _,
            }],
        );

        grr.bind_pipeline(pipeline_raster);
        grr.bind_color_blend_state(&color_blend);
        grr.bind_uniform_constants(
            pipeline_raster,
            0,
            &[
                grr::Constant::U32(gpu_data.primitives.len() as _), // primitives
                grr::Constant::Vec4(viewport.get_rect()),           // viewport
                grr::Constant::Vec2([w as f32, h as f32]),          // primitives
            ],
        );
        grr.bind_storage_buffers(
            0,
            &[
                grr::BufferRange {
                    buffer: gpu_vertices,
                    offset: 0,
                    size: (std::mem::size_of::<u32>() * gpu_data.vertices.len()) as _,
                },
                grr::BufferRange {
                    buffer: gpu_primitives,
                    offset: 0,
                    size: (std::mem::size_of::<u32>() * gpu_data.primitives.len()) as _,
                },
            ],
        );

        grr.clear_attachment(
            grr::Framebuffer::DEFAULT,
            grr::ClearAttachment::ColorFloat(0, [1.0, 1.0, 1.0, 0.0]),
        );

        grr.write_timestamp(query[0]);

        let num_vertices = gpu_data.bbox.len() as u32 / 4;
        grr.draw(grr::Primitive::Triangles, 0..num_vertices, 0..1);

        grr.write_timestamp(query[1]);

        let t0 = grr.get_query_result_u64(query[0], grr::QueryResultFlags::WAIT);
        let t1 = grr.get_query_result_u64(query[1], grr::QueryResultFlags::WAIT);

        avg_frametime_gpu.update((t1 - t0) as f32 / 1_000_000_000.0f32);

        window.swap_buffers()?;
    }

    Ok(())
}

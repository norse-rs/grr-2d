use std::error::Error;
use glutin::dpi::LogicalSize;
use rand::prelude::*;


fn main() -> Result<(), Box<Error>> {
    unsafe {
        let mut events_loop = glutin::EventsLoop::new();
        let wb = glutin::WindowBuilder::new()
            .with_title("grr - Bezier")
            .with_dimensions(LogicalSize {
                width: 1240.0,
                height: 700.0,
            });
        let window = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_srgb(true)
            .with_gl_debug_flag(false)
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

        let shader_vert = grr.create_shader(
            grr::ShaderStage::Vertex,
            include_bytes!("../assets/bezier.vert"),
        )?;
        let shader_frag = grr.create_shader(
            grr::ShaderStage::Fragment,
            include_bytes!("../assets/bezier.frag"),
        )?;
        let pipeline= grr.create_graphics_pipeline(grr::VertexPipelineDesc {
            vertex_shader: shader_vert,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: Some(shader_frag),
        })?;

        let color_target = grr.create_image(
            grr::ImageType::D2 {
                width: w as _,
                height: h as _,
                layers: 1,
                samples: 1,
            },
            grr::Format::R8G8B8A8_UNORM,
            1,
        )?;
        let color_target_view = grr.create_image_view(
            color_target,
            grr::ImageViewType::D2,
            grr::Format::R8G8B8A8_SRGB,
            grr::SubresourceRange {
                layers: 0..1,
                levels: 0..1,
            },
        )?;

        let bezier_curve = [
            -0.8f32, -0.5, 0.0, 0.0,
            -0.2, -0.5, 1.0, 1.0,
            -0.25, 0.5, 0.5, 0.0,

            0.4, -0.2, 0.5, 0.0,
            0.7, 0.0, 1.0, 1.0,
            0.25, 0.5, 0.0, 0.0,
        ];
        let bezier_data =
            grr.create_buffer_from_host(grr::as_u8_slice(&bezier_curve), grr::MemoryFlags::empty())?;

        let vertex_array = grr.create_vertex_array(&[
            grr::VertexAttributeDesc {
                location: 0,
                binding: 0,
                format: grr::VertexFormat::Xy32Float,
                offset: 0,
            },
            grr::VertexAttributeDesc {
                location: 1,
                binding: 0,
                format: grr::VertexFormat::Xy32Float,
                offset: (2 * std::mem::size_of::<f32>()) as _,
            },
        ])?;

        let present_fbo = grr.create_framebuffer()?;
        grr.set_color_attachments(present_fbo, &[0]);
        grr.bind_attachments(
            present_fbo,
            &[
                (
                    grr::Attachment::Color(0),
                    grr::AttachmentView::Image(color_target_view),
                ),
            ],
        );

        let mut running = true;

        let mut time_last = std::time::Instant::now();
        let mut avg_frametime = 0.0;

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
                _ => (),
            });

            // timing
            let time_now = std::time::Instant::now();
            let elapsed = time_now.duration_since(time_last).as_micros() as f32 / 1_000_000.0;
            time_last = time_now;
            avg_frametime *= 0.95;
            avg_frametime += 0.05 * elapsed;
            window.window().set_title(&format!(
                "grr-2d :: frame: {:.2} ms",
                avg_frametime * 1000.0
            ));

            grr.bind_pipeline(pipeline);
            grr.bind_vertex_array(vertex_array);
            grr.bind_vertex_buffers(
                vertex_array,
                0,
                &[grr::VertexBufferView {
                    buffer: bezier_data,
                    offset: 0,
                    stride: (std::mem::size_of::<f32>() * 4) as _,
                    input_rate: grr::InputRate::Vertex,
                }],
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

            grr.bind_framebuffer(present_fbo);
            grr.set_color_attachments(present_fbo, &[0]);
            grr.bind_attachments(
                present_fbo,
                &[(
                    grr::Attachment::Color(0),
                    grr::AttachmentView::Image(color_target_view),
                )],
            );
            grr.clear_attachment(present_fbo, grr::ClearAttachment::ColorFloat(0, [0.1, 0.1, 0.1, 1.0]));

            grr.draw(grr::Primitive::Triangles, 0..6, 0..1);

            let screen = grr::Region {
                x: 0,
                y: 0,
                w: w as _,
                h: h as _,
            };
            grr.blit(
                present_fbo,
                screen,
                grr::Framebuffer::DEFAULT,
                screen,
                grr::Filter::Linear,
            );
            window.swap_buffers()?;
        }
    }

    Ok(())
}

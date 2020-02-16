use glutin::dpi::LogicalSize;
use glutin::ElementState;
use grr_2d::GlyphPositioner;
use nalgebra_glm as glm;
use std::error::Error;

const ROBOTO: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        let mut events_loop = glutin::EventsLoop::new();
        let wb = glutin::WindowBuilder::new()
            .with_title("grr - lanka")
            .with_dimensions(LogicalSize {
                width: 1240.0,
                height: 700.0,
            });
        let window = glutin::ContextBuilder::new()
            .with_vsync(true)
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

        let mut gpu_data = grr_2d::GpuData::new();

        let font = grr_2d::Font::from_bytes(&ROBOTO).unwrap();
        let glyphs = grr_2d::Layout::default().calculate_glyphs(
            &[font],
            &grr_2d::SectionGeometry {
                screen_position: (0.0, 0.0),
                ..grr_2d::SectionGeometry::default()
            },
            &[grr_2d::SectionText {
                text: "ABCDE",
                scale: grr_2d::Scale::uniform(112.0),
                ..grr_2d::SectionText::default()
            }],
        );

        for (glyph, _, _) in glyphs {
            use rusttype::Segment;

            let mut path = grr_2d::PathBuilder::new();

            let bbox = glyph.unpositioned().exact_bounding_box().unwrap();
            let shapes = glyph.unpositioned().shape().unwrap();
            let mut pos = glyph.position();
            pos.y -= bbox.max.y + bbox.min.y;

            for shape in shapes {
                for segment in &shape.segments {
                    match segment {
                        Segment::Line(line) => {
                            path = path.move_to(glm::vec2(
                                line.p[0].x - bbox.min.x,
                                line.p[0].y + bbox.max.y,
                            ));
                            path = path.line_to(glm::vec2(
                                line.p[1].x - bbox.min.x,
                                line.p[1].y + bbox.max.y,
                            ));
                        }
                        Segment::Curve(curve) => {
                            path = path.move_to(glm::vec2(
                                curve.p[0].x - bbox.min.x,
                                curve.p[0].y + bbox.max.y,
                            ));
                            path = path.quad_to(
                                glm::vec2(curve.p[1].x - bbox.min.x, curve.p[1].y + bbox.max.y),
                                glm::vec2(curve.p[2].x - bbox.min.x, curve.p[2].y + bbox.max.y),
                            );
                        }
                    }
                }
            }

            let curves = path.monotonize().fill().finish();

            let rect = grr_2d::Rect {
                offset_local: glm::vec2(pos.x + bbox.min.x as f32, pos.y + bbox.min.y as f32),
                extent_local: glm::vec2(
                    bbox.max.x as f32 - bbox.min.x as f32,
                    bbox.max.y as f32 - bbox.min.y as f32,
                ),
                offset_curve: glm::vec2(0.0, 0.0),
                extent_curve: glm::vec2(
                    bbox.max.x as f32 - bbox.min.x as f32,
                    bbox.max.y as f32 - bbox.min.y as f32,
                ),
            };

            gpu_data.extend(&curves, rect);
        }

        let mut box_path = grr_2d::PathBuilder::new();
        let box_path = box_path
            .move_to(glm::vec2(-30.0, 40.0))
            .quad_to(glm::vec2(-20.0, 60.0), glm::vec2(20.0, 70.0))
            .line_to(glm::vec2(50.0, 50.0))
            .line_to(glm::vec2(30.0, 0.0))
            // .close()
            .stroke(4.0, (grr_2d::CurveCap::Round, grr_2d::CurveJoin::Bevel, grr_2d::CurveCap::Butt));
        let box_aabb = grr_2d::Aabb::from_curves(&box_path);
        gpu_data.extend(
            &box_path,
            grr_2d::Rect {
                offset_local: box_aabb.min,
                extent_local: box_aabb.max - box_aabb.min,
                offset_curve: box_aabb.min,
                extent_curve: box_aabb.max - box_aabb.min,
            },
        );

        let theta0 = std::f32::consts::PI / 180.0 * 220.0;
        let theta1 = std::f32::consts::PI / 180.0 * 180.0;

        let (sin0, cos0) = theta0.sin_cos();
        let (sin1, cos1) = theta1.sin_cos();

        let radius = 100.0;

        // let mut triangle_path = grr_2d::PathBuilder::new()
        //     .move_to(glm::vec2(0.0, 0.0))
        //     .line_to(glm::vec2(cos0 * radius, sin0 * radius))
        //     .arc_to(glm::vec2(0.0, 0.0), glm::vec2(cos1 * radius, sin1 * radius))
        //     .close()
        //     .fill().finish();
        // let triangle_aabb = grr_2d::Aabb::from_curves(&triangle_path);
        // gpu_data.extend(
        //     &triangle_path,
        //     grr_2d::Rect {
        //         offset_local: triangle_aabb.min,
        //         extent_local: triangle_aabb.max - triangle_aabb.min,
        //         offset_curve: triangle_aabb.min,
        //         extent_curve: triangle_aabb.max - triangle_aabb.min,
        //     },
        // );

        // let mut circle_path = vec![grr_2d::Curve::Circle { center: glm::vec2(0.0, 0.0), radius: 4.0 }];
        // let circle_aabb = grr_2d::Aabb::from_curves(&circle_path);
        // gpu_data.extend(
        //     &circle_path,
        //     grr_2d::Rect {
        //         offset_local: circle_aabb.min,
        //         extent_local: circle_aabb.max - circle_aabb.min,
        //         offset_curve: circle_aabb.min,
        //         extent_curve: circle_aabb.max - circle_aabb.min,
        //     },
        // );

        let gpu_vertices = grr.create_buffer_from_host(
            grr::as_u8_slice(&gpu_data.vertices),
            grr::MemoryFlags::empty(),
        )?;
        let gpu_bbox = grr
            .create_buffer_from_host(grr::as_u8_slice(&gpu_data.bbox), grr::MemoryFlags::empty())?;
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
        let mut avg_frametime = 0.0;

        let mut viewport = grr_2d::Viewport {
            position: (0.0, 0.0),
            scaling_y: h as _,
            aspect_ratio: (w / h) as _,
        };

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
            avg_frametime *= 0.95;
            avg_frametime += 0.05 * elapsed;
            window.window().set_title(&format!(
                "grr-2d :: frame: {:.2} ms",
                avg_frametime * 1000.0
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
                        size: (std::mem::size_of::<f32>() * gpu_data.vertices.len()) as _,
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

            let num_vertices = gpu_data.bbox.len() as u32 / 4;
            grr.draw(grr::Primitive::Triangles, 0..num_vertices, 0..1);

            window.swap_buffers()?;
        }
    }

    Ok(())
}

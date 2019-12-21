use std::error::Error;
use glutin::dpi::LogicalSize;
use glutin::ElementState;
use glyph_brush_layout::{SectionGeometry, SectionText, GlyphPositioner};
use rusttype::{Segment, Font, Scale};

use nalgebra_glm as glm;

const ROBOTO: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

const VIEW_SCALE: f32 = 800.0;

#[repr(u32)]
enum Primitive {
    LineField = 0x1,
    StrokeShade = 0x2,
    QuadraticField = 0x3,
    Fill = 0x4,
    QuadraticMono = 0x5,
}

struct GlyphBuilder {
    vertices: Vec<f32>,
    primitives: Vec<Primitive>,
    last: [f32; 2],
}

impl GlyphBuilder {
    fn new() -> Self {
        GlyphBuilder {
            vertices: Vec::new(),
            primitives: Vec::new(),
            last: [0.0; 2],
        }
    }
}

impl ttf_parser::OutlineBuilder for GlyphBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        println!("move {:?}", (x, y));
        self.last = [x, y];
    }

    fn line_to(&mut self, x: f32, y: f32) {
        println!("line {:?}", (x, y));
        // b0
        self.vertices.push(self.last[0]);
        self.vertices.push(self.last[1]);

        // b1
        self.vertices.push(x);
        self.vertices.push(y);

        self.primitives.push(Primitive::LineField);

        self.last = [x, y];
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        println!("quad to {:?}", (x1, y1, x, y));
        // b0
        self.vertices.push(self.last[0]);
        self.vertices.push(self.last[1]);

        // b1
        self.vertices.push(x1);
        self.vertices.push(y1);

        // b2
        self.vertices.push(x);
        self.vertices.push(y);

        // // extrema // TODO
        self.vertices.push(x);
        self.vertices.push(y);

        self.primitives.push(Primitive::QuadraticField);

        self.last = [x, y];
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        unimplemented!()
    }

    fn close(&mut self) {
        // TODO: currently just stroke
        println!("close");
    }
}

struct GlyphBuilderMonotonic {
    vertices: Vec<f32>,
    primitives: Vec<Primitive>,
    last: [f32; 2],
}

impl GlyphBuilderMonotonic {
    fn new() -> Self {
        GlyphBuilderMonotonic {
            vertices: Vec::new(),
            primitives: Vec::new(),
            last: [0.0; 2],
        }
    }
}

impl ttf_parser::OutlineBuilder for GlyphBuilderMonotonic {
    fn move_to(&mut self, x: f32, y: f32) {
        println!("move {:?}", (x, y));
        self.last = [x, y];
    }

    fn line_to(&mut self, x: f32, y: f32) {
        println!("line {:?}", (x, y));
        // b0
        self.vertices.push(self.last[0]);
        self.vertices.push(self.last[1]);

        // b1
        self.vertices.push(x);
        self.vertices.push(y);

        self.primitives.push(Primitive::LineField);

        self.last = [x, y];
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        println!("quad to {:?}", (x1, y1, x, y));

        let p0 = glm::vec2(self.last[0], self.last[1]);
        let p1 = glm::vec2(x1, y1);
        let p2 = glm::vec2(x, y);

        let min = glm::min2(&p0, &p2);
        let max = glm::max2(&p0, &p2);

        if p1[0] < min[0] || p1[0] > max[0] || p1[1] < min[1] || p1[1] > max[1] {
            let t0 = (p0[0] - p1[0]) / (p0[0] - 2.0 * p1[0] + p2[0]);
            let t1 = (p0[1] - p1[1]) / (p0[1] - 2.0 * p1[1] + p2[1]);

            unimplemented!()
        } else {
            self.vertices.push(self.last[0]);
            self.vertices.push(self.last[1]);

            // b1
            self.vertices.push(x1);
            self.vertices.push(y1);

            // b2
            self.vertices.push(x);
            self.vertices.push(y);

            self.primitives.push(Primitive::QuadraticMono);
        }

        self.last = [x, y];
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        unimplemented!()
    }

    fn close(&mut self) {
        // TODO: currently just stroke
        println!("close");
    }
}

#[derive(Copy, Clone, Debug)]
struct Viewport {
    position: (f32, f32),
    scaling_y: f32,
    aspect_ratio: f32, // width / height
}

impl Viewport {
    fn get_rect(&self) -> [f32; 4] {
        let (sx, sy) = self.get_scale();

        let (cx, cy) = self.position;
        let x = cx - sx / 2.0;
        let y = cy - sy / 2.0;

        [x, y, sx, sy]
    }

    fn get_scale(&self) -> (f32, f32) {
        (self.scaling_y * self.aspect_ratio * VIEW_SCALE, self.scaling_y * VIEW_SCALE)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let TILE_SIZE_X = 32;
    let TILE_SIZE_Y = 8;

    unsafe {
        let mut events_loop = glutin::EventsLoop::new();
        let wb = glutin::WindowBuilder::new()
            .with_title("grr - Panthera")
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

        let font = Font::from_bytes(&ROBOTO)?;
        let glyphs = glyph_brush_layout::Layout::default().calculate_glyphs(
            &[font],
            &SectionGeometry {
                screen_position: (0.0, 0.0),
                ..SectionGeometry::default()
            },
            &[
                SectionText {
                    text: "Test?",
                    scale: Scale::uniform(100.0),
                    ..SectionText::default()
                },
            ],
        );

        let mut glyph_bbox_triangles = Vec::<f32>::new();
        let mut glyph_vertices = Vec::<f32>::new();
        let mut glyph_primitives = Vec::<Primitive>::new();
        for (glyph, _, font) in glyphs {
            let bbox = glyph.unpositioned().exact_bounding_box().unwrap();
            let shapes = glyph.unpositioned().shape().unwrap();
            let mut pos = glyph.position();
            pos.y -= bbox.min.y;

            dbg!(&bbox);

            glyph_bbox_triangles.extend(&[
                pos.x + bbox.min.x as f32, pos.y  + bbox.min.y as f32, pos.x + bbox.min.x as f32, pos.y + bbox.min.y as f32,
                pos.x + bbox.min.x as f32, pos.y  + bbox.max.y as f32, pos.x + bbox.min.x as f32, pos.y + bbox.max.y as f32,
                pos.x + bbox.max.x as f32, pos.y  + bbox.max.y as f32, pos.x + bbox.max.x as f32, pos.y + bbox.max.y as f32,
                pos.x + bbox.max.x as f32, pos.y  + bbox.min.y as f32, pos.x + bbox.max.x as f32, pos.y + bbox.min.y as f32,
                pos.x + bbox.min.x as f32, pos.y  + bbox.min.y as f32, pos.x + bbox.min.x as f32, pos.y + bbox.min.y as f32,
                pos.x + bbox.max.x as f32, pos.y  + bbox.max.y as f32, pos.x + bbox.max.x as f32, pos.y + bbox.max.y as f32,
            ]);

            for shape in shapes {
                for segment in shape.segments {
                    match segment {
                        Segment::Line(line) => {
                            glyph_vertices.push(pos.x + line.p[0].x);
                            glyph_vertices.push(pos.y + line.p[0].y + bbox.min.y + bbox.max.y);

                            glyph_vertices.push(pos.x + line.p[1].x);
                            glyph_vertices.push(pos.y + line.p[1].y + bbox.min.y + bbox.max.y);

                            glyph_primitives.push(Primitive::LineField);
                        }
                        Segment::Curve(curve) => {
                            let p0 = glm::vec2(pos.x + curve.p[0].x, pos.y + curve.p[0].y + bbox.min.y  + bbox.max.y);
                            let p1 = glm::vec2(pos.x + curve.p[1].x, pos.y + curve.p[1].y + bbox.min.y  + bbox.max.y);
                            let p2 = glm::vec2(pos.x + curve.p[2].x, pos.y + curve.p[2].y + bbox.min.y  + bbox.max.y);

                            let min = glm::min2(&p0, &p2);
                            let max = glm::max2(&p0, &p2);

                            if p1[0] < min[0] || p1[0] > max[0] || p1[1] < min[1] || p1[1] > max[1] {
                                let t0 = (p0[0] - p1[0]) / (p0[0] - 2.0 * p1[0] + p2[0]);
                                let t1 = (p0[1] - p1[1]) / (p0[1] - 2.0 * p1[1] + p2[1]);

                                unimplemented!()
                            } else {
                                glyph_vertices.push(p0[0]);
                                glyph_vertices.push(p0[1]);

                                // b1
                                glyph_vertices.push(p1[0]);
                                glyph_vertices.push(p1[1]);

                                // b2
                                glyph_vertices.push(p2[0]);
                                glyph_vertices.push(p2[1]);

                                glyph_primitives.push(Primitive::QuadraticMono);
                            }
                        }
                    }
                }
            }
        }

        glyph_primitives.push(Primitive::StrokeShade);

        // let glyph_a_id = font.glyph_index('?')?;
        // let mut glyph_a = GlyphBuilderMonotonic::new();
        // let glyph_a_bbox = font.outline_glyph(glyph_a_id, &mut glyph_a).unwrap();
        // glyph_a.primitives.push(Primitive::StrokeShade);
        // println!("{:?}", glyph_a_bbox);
        // println!("{:?}", font.units_per_em());

        // let factor = 0.3;

        // let glyph_bbox_triangles = [
        //     factor * glyph_a_bbox.x_min as f32, factor * glyph_a_bbox.y_min as f32, glyph_a_bbox.x_min as f32, glyph_a_bbox.y_min as f32,
        //     factor * glyph_a_bbox.x_min as f32, factor * glyph_a_bbox.y_max as f32, glyph_a_bbox.x_min as f32, glyph_a_bbox.y_max as f32,
        //     factor * glyph_a_bbox.x_max as f32, factor * glyph_a_bbox.y_max as f32, glyph_a_bbox.x_max as f32, glyph_a_bbox.y_max as f32,
        //     factor * glyph_a_bbox.x_max as f32, factor * glyph_a_bbox.y_min as f32, glyph_a_bbox.x_max as f32, glyph_a_bbox.y_min as f32,
        //     factor * glyph_a_bbox.x_min as f32, factor * glyph_a_bbox.y_min as f32, glyph_a_bbox.x_min as f32, glyph_a_bbox.y_min as f32,
        //     factor * glyph_a_bbox.x_max as f32, factor * glyph_a_bbox.y_max as f32, glyph_a_bbox.x_max as f32, glyph_a_bbox.y_max as f32,
        // ];
        let glyph_bbox_triangles_data =
            grr.create_buffer_from_host(grr::as_u8_slice(&glyph_bbox_triangles), grr::MemoryFlags::empty())?;

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
                offset: 2 * std::mem::size_of::<f32>() as u32,
            },
        ])?;

        let shader_vs = grr.create_shader(
            grr::ShaderStage::Vertex,
            include_bytes!("../assets/lupis.vs"),
        )?;
        let shader_fs = grr.create_shader(
            grr::ShaderStage::Fragment,
            include_bytes!("../assets/lupis.fs"),
        )?;
        let pipeline_raster = grr.create_graphics_pipeline(grr::VertexPipelineDesc {
            vertex_shader: shader_vs,
            geometry_shader: None,
            tessellation_evaluation_shader: None,
            tessellation_control_shader: None,
            fragment_shader: Some(shader_fs),
        })?;

        let color_target = grr.create_image(
            grr::ImageType::D2 {
                width: w as _,
                height: h as _,
                layers: 1,
                samples: 1,
            },
            grr::Format::R16G16B16A16_SFLOAT,
            1,
        )?;
        let color_target_view = grr.create_image_view(
            color_target,
            grr::ImageViewType::D2,
            grr::Format::R16G16B16A16_SFLOAT,
            grr::SubresourceRange {
                layers: 0..1,
                levels: 0..1,
            },
        )?;

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

        let num_tiles_x = (w as u32 + TILE_SIZE_X - 1) / TILE_SIZE_X;
        let num_tiles_y = (h as u32 + TILE_SIZE_Y - 1) / TILE_SIZE_Y;

        let scene_vertices = grr.create_buffer_from_host(grr::as_u8_slice(&glyph_vertices), grr::MemoryFlags::DEVICE_LOCAL)?;
        let scene_primitives = grr.create_buffer_from_host(grr::as_u8_slice(&glyph_primitives), grr::MemoryFlags::DEVICE_LOCAL)?;

        let timer_query = grr.create_query(grr::QueryType::TimeElapsed);

        let mut viewport = Viewport {
            position: (136.55, 115.65),
            scaling_y: 1.0,
            aspect_ratio: (w / h) as _,
        };

        let mut mouse1 = ElementState::Released;

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
                    event: glutin::DeviceEvent::MouseWheel { delta: glutin::MouseScrollDelta::LineDelta(_, delta) },
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

            grr.bind_framebuffer(present_fbo);

            grr.bind_vertex_array(vertex_array);
            grr.bind_vertex_buffers(
                vertex_array,
                0,
                &[grr::VertexBufferView {
                    buffer: glyph_bbox_triangles_data,
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

            grr.bind_pipeline(pipeline_raster);
            grr.bind_color_blend_state(&color_blend);
            grr.bind_uniform_constants(
                pipeline_raster,
                0,
                &[
                    grr::Constant::U32(glyph_primitives.len() as _), // primitives
                    grr::Constant::Vec4(viewport.get_rect()), // viewport
                    grr::Constant::Vec2([w as f32, h as f32]), // primitives
                ],
            );
            grr.bind_storage_buffers(0, &[
                grr::BufferRange {
                    buffer: scene_vertices,
                    offset: 0,
                    size: (std::mem::size_of::<f32>() * glyph_vertices.len()) as _,
                },
                grr::BufferRange {
                    buffer: scene_primitives,
                    offset: 0,
                    size: (std::mem::size_of::<u32>() * glyph_primitives.len()) as _,
                },
            ]);

            grr.clear_attachment(
                present_fbo,
                grr::ClearAttachment::ColorFloat(0, [1.0, 1.0, 1.0, 0.0]),
            );

            grr.begin_query(&timer_query);
            let num_vertices = glyph_bbox_triangles.len() as u32 / 4;
            grr.draw(grr::Primitive::Triangles, 0..num_vertices, 0..1);
            grr.end_query(&timer_query);

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

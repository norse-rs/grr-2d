use glutin::dpi::LogicalSize;
use glutin::ElementState;
use std::error::Error;
use std::fmt::Write;

const ROBOTO: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

#[repr(u32)]
enum Primitive {
    LineField = 0x1,
    StrokeShade = 0x2,
    QuadraticField = 0x3,
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
        self.last = [x, y];
    }

    fn line_to(&mut self, x: f32, y: f32) {
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
        // b0
        self.vertices.push(self.last[0]);
        self.vertices.push(self.last[1]);

        // b1
        self.vertices.push(x1);
        self.vertices.push(y1);

        // b2
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
        self.primitives.push(Primitive::StrokeShade);
    }
}

struct Viewport {
    position: (f32, f32),
    scaling_y: f32,
    aspect_ratio: f32, // width / height
}

impl Viewport {
    fn get_rect(&self) -> [f32; 4] {
        let height = self.scaling_y;
        let width = self.aspect_ratio * height;

        let (cx, cy) = self.position;
        let x = cx - width / 2.0;
        let y = cy - height / 2.0;

        [x, y, width, height]
    }

    fn get_scale(&self) -> (f32, f32) {
        (self.scaling_y * self.aspect_ratio, self.scaling_y)
    }
}

fn main() -> Result<(), Box<Error>> {
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

        let font = ttf_parser::Font::from_data(&ROBOTO, 0)?;
        let glyph_a_id = font.glyph_index('φ')?;
        let mut glyph_a = GlyphBuilder::new();
        let glyph_a_bbox = font.outline_glyph(glyph_a_id, &mut glyph_a).unwrap();
        println!("{:?}", glyph_a_bbox);
        println!("{:?}", font.units_per_em());

        let shader_coarse_culling = grr.create_shader(
            grr::ShaderStage::Compute,
            include_bytes!("../assets/panta_culling.comp"),
        )?;
        let shader_raster = grr.create_shader(
            grr::ShaderStage::Compute,
            include_bytes!("../assets/panta_raster.comp"),
        )?;
        let pipeline_raster = grr.create_compute_pipeline(shader_raster)?;

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

        let present_fbo = grr.create_framebuffer()?;
        grr.set_color_attachments(present_fbo, &[0]);
        grr.bind_attachments(
            present_fbo,
            &[(
                grr::Attachment::Color(0),
                grr::AttachmentView::Image(color_target_view),
            )],
        );

        let mut running = true;

        let mut time_last = std::time::Instant::now();
        let mut avg_frametime = 0.0;

        let num_tiles_x = (w as u32 + TILE_SIZE_X - 1) / TILE_SIZE_X;
        let num_tiles_y = (h as u32 + TILE_SIZE_Y - 1) / TILE_SIZE_Y;

        // let vertices = [
        //     40.0f32, 20.0f32,
        //     80.0f32, 60.0f32,
        //     150.0, 10.0
        // ];
        // let primitives = [3u32, 2];

        // let vertices = [
        //     -2.5f32, 6.25,
        //     0.0, -6.25,
        //     2.5, 6.25,
        // ];
        // let primitives = [3u32];

        // let vertices = [
        //     40.0f32, 20.0f32, 80.0f32, 60.0f32,
        //     80.0f32, 60.0f32, 150.0, 10.0
        // ];
        // let primitives = [1u32, 1, 2];

        let scene_vertices = grr.create_buffer_from_host(
            grr::as_u8_slice(&glyph_a.vertices),
            grr::MemoryFlags::DEVICE_LOCAL,
        )?;
        let scene_primitives = grr.create_buffer_from_host(
            grr::as_u8_slice(&glyph_a.primitives),
            grr::MemoryFlags::DEVICE_LOCAL,
        )?;

        let timer_query = grr.create_query(grr::QueryType::TimeElapsed);

        let mut viewport = Viewport {
            position: (100.0, 40.0),
            scaling_y: 100.0,
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
                        viewport.position.1 += scale.1 * (delta.1 / h) as f32
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

            grr.bind_pipeline(pipeline_raster);
            grr.bind_uniform_constants(
                pipeline_raster,
                0,
                &[
                    grr::Constant::U32(glyph_a.primitives.len() as _), // primitives
                    grr::Constant::Vec4(viewport.get_rect()),          // primitives
                ],
            );
            grr.bind_storage_buffers(
                0,
                &[
                    grr::BufferRange {
                        buffer: scene_vertices,
                        offset: 0,
                        size: (std::mem::size_of::<f32>() * glyph_a.vertices.len()) as _,
                    },
                    grr::BufferRange {
                        buffer: scene_primitives,
                        offset: 0,
                        size: (std::mem::size_of::<u32>() * glyph_a.primitives.len()) as _,
                    },
                ],
            );
            grr.bind_storage_image_views(5, &[color_target_view]);

            grr.begin_query(&timer_query);
            grr.dispatch(num_tiles_x, num_tiles_y, 1);
            grr.end_query(&timer_query);

            grr.memory_barrier(grr::Barrier::FRAMEBUFFER_RW);

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

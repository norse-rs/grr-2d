use std::error::Error;
use glutin::dpi::LogicalSize;
use rand::prelude::*;

const SINGLE_PASS: bool = true;

const MAX_NUM_TILES_X: u64 = 64;
const MAX_NUM_TILES_Y: u64 = 128;
const MAX_TILE_SIZE: u64 = 8192;

#[repr(C)]
struct CmdCircle {
    ty: u32,
    color: u32,
}

fn main() -> Result<(), Box<Error>> {
    let TILE_SIZE_X = if SINGLE_PASS { 8} else { 32 };
    let TILE_SIZE_Y = if SINGLE_PASS { 4 } else { 8 };

    unsafe {
        let mut events_loop = glutin::EventsLoop::new();
        let wb = glutin::WindowBuilder::new()
            .with_title("grr - Circles")
            .with_dimensions(LogicalSize {
                width: 1440.0,
                height: 700.0,
            });
        let window = glutin::ContextBuilder::new()
            .with_vsync(false)
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

        let shader_tiler = grr.create_shader(
            grr::ShaderStage::Compute,
            include_bytes!("../assets/tiler.comp"),
        )?;
        let shader_rasterizer = grr.create_shader(
            grr::ShaderStage::Compute,
            include_bytes!("../assets/rasterizer.comp"),
        )?;
        let shader_single_pass = grr.create_shader(
            grr::ShaderStage::Compute,
            include_bytes!("../assets/single_pass.comp"),
        )?;

        let pipeline_tiler = grr.create_compute_pipeline(shader_tiler)?;
        let pipeline_rasterizer = grr.create_compute_pipeline(shader_rasterizer)?;
        let pipeline_single_pass = grr.create_compute_pipeline(shader_single_pass)?;

        let tile_cmd_buffer = grr.create_buffer(MAX_NUM_TILES_X * MAX_NUM_TILES_Y * MAX_TILE_SIZE, grr::MemoryFlags::DEVICE_LOCAL)?;

        let mut rng = rand::thread_rng();

        let mut bboxes: Vec<[f32; 4]> = Vec::new();
        let mut circles: Vec<CmdCircle> = Vec::new();
        let mut rng = rand::thread_rng();
        for n in 0..1000 {
            let diameter: u16 = rng.gen_range(20, 200);
            let bbox_min_x: u16 = rng.gen_range(0, 770.0 as u16);
            let bbox_min_y: u16 = rng.gen_range(0, 350.0 as u16);

            bboxes.push([bbox_min_x as _, bbox_min_y as _, (bbox_min_x + diameter) as _, (bbox_min_y + diameter) as _]);
            circles.push(CmdCircle {
                ty: 0x1,
                color: 0x40FF0000,
            });
        }

        // let bboxes = [
        //     [10.0f32, 20.0, 170.0, 180.0],
        //     [180.0, 110.0, 220.0, 150.0],
        //     [260.0, 260.0, 340.0, 340.0],
        //     [20.0, 10.0, 100.0, 90.0],
        // ];
        // let circles = [
        //     CmdCircle {
        //         ty: 0x1,
        //         color: 0x60FF0000,
        //     },
        //     CmdCircle {
        //         ty: 0x1,
        //         color: 0x2000FF00,
        //     },
        //     CmdCircle {
        //         ty: 0x1,
        //         color: 0xFF0000FF,
        //     },
        //     CmdCircle {
        //         ty: 0x1,
        //         color: 0x404AB020,
        //     },
        // ];

        let scene_bboxes = grr.create_buffer_from_host(grr::as_u8_slice(&bboxes), grr::MemoryFlags::DEVICE_LOCAL)?;
        let scene_buffer = grr.create_buffer_from_host(grr::as_u8_slice(&circles), grr::MemoryFlags::DEVICE_LOCAL)?;

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

            let num_tiles_x = (w as u32 + TILE_SIZE_X - 1) / TILE_SIZE_X;
            let num_tiles_y = (h as u32 + TILE_SIZE_Y - 1) / TILE_SIZE_Y;

            let num_tile_groups_x = (num_tiles_x + 31) / 32;

            if !SINGLE_PASS {
                grr.bind_pipeline(pipeline_tiler);
                grr.bind_storage_buffers(0, &[
                    grr::BufferRange {
                        buffer: scene_bboxes,
                        offset: 0,
                        size: (4 * std::mem::size_of::<f32>() * bboxes.len()) as _,
                    },
                    // entities
                    grr::BufferRange {
                        buffer: scene_buffer,
                        offset: 0,
                        size: (std::mem::size_of::<CmdCircle>() * circles.len()) as _,
                    },
                    // circles
                    grr::BufferRange {
                        buffer: scene_buffer,
                        offset: 0,
                        size: (std::mem::size_of::<CmdCircle>() * circles.len()) as _,
                    },
                    //
                    grr::BufferRange {
                        buffer: tile_cmd_buffer,
                        offset: 0,
                        size: (MAX_NUM_TILES_X * MAX_NUM_TILES_Y * MAX_TILE_SIZE) as _,
                    },
                ]);

                grr.bind_uniform_constants(
                    pipeline_tiler,
                    0,
                    &[
                        grr::Constant::U32(circles.len() as _),
                    ],
                );

                grr.bind_storage_image_views(0, &[color_target_view]);
                grr.dispatch(num_tile_groups_x, num_tiles_y, 1);

                grr.memory_barrier(grr::Barrier::STORAGE_BUFFER_RW);

                grr.bind_pipeline(pipeline_rasterizer);

                grr.bind_storage_buffers(0, &[
                    grr::BufferRange {
                        buffer: tile_cmd_buffer,
                        offset: 0,
                        size: (MAX_NUM_TILES_X * MAX_NUM_TILES_Y * MAX_TILE_SIZE) as _,
                    },
                ]);

                grr.bind_storage_image_views(0, &[color_target_view]);
                grr.dispatch(num_tiles_x, num_tiles_y, 1);
            } else {
                grr.bind_pipeline(pipeline_single_pass);
                grr.bind_storage_buffers(0, &[
                    grr::BufferRange {
                        buffer: scene_bboxes,
                        offset: 0,
                        size: (4 * std::mem::size_of::<f32>() * bboxes.len()) as _,
                    },
                    // entities
                    grr::BufferRange {
                        buffer: scene_buffer,
                        offset: 0,
                        size: (std::mem::size_of::<CmdCircle>() * circles.len()) as _,
                    },
                    // circles
                    grr::BufferRange {
                        buffer: scene_buffer,
                        offset: 0,
                        size: (std::mem::size_of::<CmdCircle>() * circles.len()) as _,
                    },
                ]);
                grr.bind_storage_image_views(0, &[color_target_view]);
                grr.bind_uniform_constants(
                    pipeline_single_pass,
                    0,
                    &[
                        grr::Constant::U32(circles.len() as _),
                    ],
                );
                grr.dispatch(num_tiles_x, num_tiles_y, 1);
            }

            grr.memory_barrier(grr::Barrier::FRAMEBUFFER_RW);

            grr.set_color_attachments(present_fbo, &[0]);
            grr.bind_attachments(
                present_fbo,
                &[(
                    grr::Attachment::Color(0),
                    grr::AttachmentView::Image(color_target_view),
                )],
            );

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

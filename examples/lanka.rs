use grr_2d::GlyphPositioner;
use nalgebra_glm as glm;
use std::error::Error;

const ROBOTO: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");

fn main() -> Result<(), Box<dyn Error>> {
    let mut gpu_data = grr_2d::GpuData::new();

    let font = grr_2d::Font::from_bytes(&ROBOTO).unwrap();
    let glyphs = grr_2d::Layout::default().calculate_glyphs(
        &[font],
        &grr_2d::SectionGeometry {
            screen_position: (0.0, 0.0),
            ..grr_2d::SectionGeometry::default()
        },
        &[grr_2d::SectionText {
            text: "The quick brown fox jumps over the lazy dog",
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

        gpu_data.extend(
            &curves,
            rect,
            &grr_2d::Brush::Color([0, 0, 0, 255])
            // &grr_2d::Brush::LinearGradient {
            //     stop0: grr_2d::GradientStop {
            //         position: glm::vec2(0.0, 80.0),
            //         color: [255, 100, 0, 255],
            //     },
            //     stop1: grr_2d::GradientStop {
            //         position: glm::vec2(0.0, 150.0),
            //         color: [255, 0, 70, 255],
            //     },
            // },
        );
    }

    let box_path = grr_2d::PathBuilder::new();
    let box_path = box_path
        .move_to(glm::vec2(0.0, 0.0))
        .quad_to(glm::vec2(100.0, 200.0), glm::vec2(140.0, 100.0))
        // .move_to(glm::vec2(-30.0, 40.0))
        // .quad_to(glm::vec2(-20.0, 60.0), glm::vec2(20.0, 70.0))
        // .line_to(glm::vec2(50.0, 50.0))
        // .line_to(glm::vec2(30.0, 0.0))
        .monotonize()
        .stroke(
            20.0,
            (
                grr_2d::CurveCap::Butt,
                grr_2d::CurveJoin::Round,
                grr_2d::CurveCap::Butt,
            ),
        );
    let box_aabb = grr_2d::Aabb::from_curves(&box_path);
    gpu_data.extend(
        &box_path,
        dbg!(dbg!(grr_2d::Rect {
            offset_local: box_aabb.min,
            extent_local: box_aabb.max - box_aabb.min,
            offset_curve: box_aabb.min,
            extent_curve: box_aabb.max - box_aabb.min,
        })
        .extrude(10.0)),
        &grr_2d::Brush::Color([255, 0, 0, 255]),
    );

    let rect_path = [grr_2d::Curve::Rect {
        p0: glm::vec2(30.0, 20.0),
        p1: glm::vec2(200.0, 50.0),
    }];
    let rect_aabb = grr_2d::Aabb::from_curves(&rect_path);
    gpu_data.extend(
        &rect_path,
        grr_2d::Rect {
            offset_local: rect_aabb.min,
            extent_local: rect_aabb.max - rect_aabb.min,
            offset_curve: rect_aabb.min,
            extent_curve: rect_aabb.max - rect_aabb.min,
        }
        .extrude(20.0),
        &grr_2d::Brush::Color([100, 100, 200, 255]),
    );

    unsafe { grr_2d::run("lanka", || { gpu_data.clone() } ) }
}

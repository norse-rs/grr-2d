use grr_2d::Extent;
use nalgebra_glm as glm;
use random_color::{Luminosity, RandomColor};
use std::error::Error;

type WidgetId = usize;

const INVALID: WidgetId = !0;
enum Phase {
    Layout { size: f32 },
}

struct Ui {
    widgets: Vec<WidgetData>,
    phase: Option<Phase>,
}

impl Ui {
    pub fn new() -> Self {
        Ui {
            widgets: Vec::new(),
            phase: None,
        }
    }

    pub fn begin_phase(&mut self, phase: Phase) -> WidgetId {
        assert!(self.phase.is_none());

        self.phase = Some(phase);
        self.widgets.clear();
        self.add_layout(Layout { flex: 1.0, min_width: 0.0, padding: 0.0 })
    }

    pub fn end_phase(&mut self) {
        match self.phase {
            Some(Phase::Layout { size }) => {
                self.layout_node(
                    0,
                    Constraint {
                        min_width: 0.0,
                        max_width: size,
                    },
                );
            }
            None => panic!("no phase started"),
        }
    }

    fn add_layout(&mut self, layout: Layout) -> WidgetId {
        let data = WidgetData {
            layout,
            rect: Rect::new(),
            first_child: INVALID,
            last_child: INVALID,
            next: INVALID,
        };

        let id = self.widgets.len();
        self.widgets.push(data);
        id
    }

    pub fn add(&mut self, parent: WidgetId, widget: impl Widget) -> WidgetId {
        match self.phase {
            Some(Phase::Layout { .. }) => {
                let child = self.add_layout(widget.layout());
                let prev_child = self.widgets[parent].last_child;
                self.widgets[parent].last_child = child;

                match prev_child {
                    INVALID => {
                        self.widgets[parent].first_child = child;
                    }
                    _ => {
                        self.widgets[prev_child].next = child;
                    }
                }

                child
            }
            _ => self.add_layout(Layout { flex: 0.0, min_width: 0.0, padding: 0.0 }) // dummy
        }
    }

    fn layout_node(&mut self, node: WidgetId, constraint: Constraint) {
        if self.widgets[node].first_child == INVALID {
            let width = if self.widgets[node].layout.flex == 0.0 {
                self.widgets[node].layout.min_width + 2.0 * self.widgets[node].layout.padding
            } else {
                (self.widgets[node]
                    .layout
                    .min_width + 2.0 * self.widgets[node].layout.padding)
                    .max(constraint.max_width)
            };
            self.widgets[node].rect.extent = width;
            return;
        }

        let mut min_width = 0.0;
        let flex_weight_sum = {
            let mut sum = 0.0;
            let mut child = self.widgets[node].first_child;
            while child != INVALID {
                sum += self.widgets[child].layout.flex;
                child = self.widgets[child].next;
            }
            sum
        };

        let mut width = 0.0;

        // non flex children
        {
            let mut child = self.widgets[node].first_child;
            while child != INVALID {
                if self.widgets[child].layout.flex == 0.0 {
                    self.layout_node(
                        child,
                        Constraint {
                            min_width: 0.0,
                            max_width: std::f32::INFINITY,
                        },
                    );

                    min_width += self.widgets[child].rect.extent;
                    width += self.widgets[child].rect.extent;
                }

                child = self.widgets[child].next;
            }
        }

        assert!(min_width < std::f32::INFINITY);
        let total_flex_available = dbg!((constraint.max_width - min_width - 2.0 * self.widgets[node].layout.padding).max(0.0));
        let mut flex_available = total_flex_available;

        // flex children
        {
            let mut child = self.widgets[node].first_child;
            while child != INVALID {
                if self.widgets[child].layout.flex != 0.0 {
                    let child_width = (total_flex_available * self.widgets[child].layout.flex
                        / flex_weight_sum)
                        .min(flex_available);
                    self.layout_node(
                        child,
                        Constraint {
                            min_width: child_width,
                            max_width: child_width,
                        },
                    );
                    width += self.widgets[child].rect.extent;
                    flex_available -= self.widgets[child].rect.extent;
                }

                child = self.widgets[child].next;
            }
        }

        self.widgets[node].rect.extent = width + 2.0 * self.widgets[node].layout.padding;
    }

    pub fn render_layout(&self, id: WidgetId, mut offset: f32, gpu_data: &mut grr_2d::GpuData) {
        let widget = &self.widgets[id];
        let rect_path = [grr_2d::Curve::Rect {
            p0: glm::vec2(offset + widget.layout.padding, widget.layout.padding),
            p1: glm::vec2(offset + widget.rect.extent - widget.layout.padding, 50.0 - widget.layout.padding),
        }];
        let rect_aabb = grr_2d::Aabb::from_curves(&rect_path);
        let c = id as u8 * 10;

        gpu_data.extend(
            &rect_path,
            grr_2d::Rect {
                offset_local: rect_aabb.min,
                extent_local: rect_aabb.max - rect_aabb.min,
                offset_curve: rect_aabb.min,
                extent_curve: rect_aabb.max - rect_aabb.min,
            }
            .extrude(2.0),
            &grr_2d::Brush::Color([c, c, c, 255]),
        );

        dbg!((id, offset, widget.rect.extent));

        let mut child = self.widgets[id].first_child;
        while child != INVALID {
            dbg!((id, child));
            self.render_layout(child, offset, gpu_data);
            offset += self.widgets[child].rect.extent;
            child = self.widgets[child].next;
        }
    }
}

trait Widget {
    fn layout(&self) -> Layout;
}

struct WidgetData {
    layout: Layout,
    rect: Rect,
    first_child: WidgetId,
    last_child: WidgetId,
    next: WidgetId,
}

struct Layout {
    flex: f32,
    min_width: f32,
    padding: f32,
}

struct Constraint {
    pub min_width: f32,
    pub max_width: f32,
}

struct Rect {
    extent: f32,
}

impl Rect {
    pub fn new() -> Self {
        Rect { extent: 0.0 }
    }
}

pub enum Flex {
    Flex(f32),
    Static(f32),
    Padding(f32),
}

impl Widget for Flex {
    fn layout(&self) -> Layout {
        match *self {
            Flex::Flex(flex) => Layout {
                flex,
                min_width: 0.0,
                padding: 0.0,
            },
            Flex::Static(w) => Layout {
                flex: 0.0,
                min_width: w,
                padding: 0.0,
            },
            Flex::Padding(p) => Layout {
                flex: 1.0,
                min_width: 0.0,
                padding: p,
            }
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut gpu_data = grr_2d::GpuData::new();

    let mut ui = Ui::new();

    let root = ui.begin_phase(Phase::Layout { size: 400.0 });
    let n0 = ui.add(root, Flex::Flex(1.0));
    ui.add(n0, Flex::Static(40.0));
    let n01 = ui.add(n0, Flex::Flex(1.0));
    ui.add(n01, Flex::Flex(3.0));
    ui.add(n01, Flex::Flex(1.0));
    let n02 = ui.add(n0, Flex::Flex(1.0));
    ui.add(n02, Flex::Padding(5.0));
    ui.end_phase();

    ui.render_layout(root, 0.0, &mut gpu_data);

    unsafe { grr_2d::run("layout", gpu_data) }
}


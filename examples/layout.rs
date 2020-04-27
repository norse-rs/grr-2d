use grr_2d::{Extent, Offset};
use nalgebra_glm as glm;
use random_color::{Luminosity, RandomColor};
use std::error::Error;

type WidgetId = usize;

const INVALID: WidgetId = !0;

#[derive(Copy, Clone, Debug)]
enum Phase {
    Update,
    Layout { size: Extent },
    Render,
    DebugLayout,
}

pub enum Event {
    MouseMove { x: f64, y: f64 },
}
// Single Pass
struct UpdateWidgets {
    num_widgets: usize,
}

impl UpdateWidgets {
    fn new() -> Self {
        UpdateWidgets { num_widgets: 0 }
    }

    fn reset(&mut self) {
        self.num_widgets = 0;
    }
}

// Multi Pass
struct LayoutWidgets {
    widgets: Vec<LayoutData>,
}

struct LayoutData {
    layout: Layout,
    rect: Rect,
    first_child: WidgetId,
    last_child: WidgetId,
    next: WidgetId,
}

impl LayoutWidgets {
    fn new() -> Self {
        LayoutWidgets {
            widgets: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.widgets.clear();
    }

    fn add(&mut self, layout: Layout) -> WidgetId {
        let data = LayoutData {
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

    fn layout_node(&mut self, node: WidgetId, constraint: Constraint) {
        if self.widgets[node].first_child == INVALID {
            let padding = glm::vec2(
                2.0 * self.widgets[node].layout.padding,
                2.0 * self.widgets[node].layout.padding,
            );
            let extent = if self.widgets[node].layout.flex == 0.0 {
                self.widgets[node].layout.extent + padding
            } else {
                glm::max2(
                    &(self.widgets[node].layout.extent + padding),
                    &constraint.max,
                )
            };
            self.widgets[node].rect.extent = extent;
            return;
        }

        let flex_weight_sum = {
            let mut sum = 0.0;
            let mut child = self.widgets[node].first_child;
            while child != INVALID {
                sum += self.widgets[child].layout.flex;
                child = self.widgets[child].next;
            }
            sum
        };

        let axis = self.widgets[node].layout.axis;

        let mut cross = axis.cross(constraint.min);
        let mut min_main = 0.0;

        // non flex children
        {
            let mut child = self.widgets[node].first_child;
            while child != INVALID {
                if self.widgets[child].layout.flex == 0.0 {
                    match axis {
                        Axis::Horizontal => {
                            self.layout_node(
                                child,
                                Constraint {
                                    min: glm::vec2(0.0, constraint.min.y),
                                    max: glm::vec2(std::f32::INFINITY, constraint.max.y),
                                },
                            );
                            min_main += self.widgets[child].rect.extent.x;
                            cross = cross.max(self.widgets[child].rect.extent.y);
                        }
                        Axis::Vertical => {
                            self.layout_node(
                                child,
                                Constraint {
                                    min: glm::vec2(constraint.min.x, 0.0),
                                    max: glm::vec2(constraint.max.x, std::f32::INFINITY),
                                },
                            );
                            min_main += self.widgets[child].rect.extent.y;
                            cross = cross.max(self.widgets[child].rect.extent.x);
                        }
                    }
                }

                child = self.widgets[child].next;
            }
        }

        assert!(min_main < std::f32::INFINITY);
        let total_flex_available =
            (axis.main(constraint.max) - min_main - 2.0 * self.widgets[node].layout.padding)
                .max(0.0);
        let mut flex_available = total_flex_available;

        let mut main = min_main;

        // flex children
        {
            let mut child = self.widgets[node].first_child;
            while child != INVALID {
                if self.widgets[child].layout.flex != 0.0 {
                    let child_main = (total_flex_available * self.widgets[child].layout.flex
                        / flex_weight_sum)
                        .min(flex_available);

                    match axis {
                        Axis::Horizontal => {
                            self.layout_node(
                                child,
                                Constraint {
                                    min: glm::vec2(child_main, constraint.min.y),
                                    max: glm::vec2(child_main, constraint.max.y),
                                },
                            );
                            main += self.widgets[child].rect.extent.x;
                            cross = cross.max(self.widgets[child].rect.extent.y);
                            flex_available -= self.widgets[child].rect.extent.x;
                        }
                        Axis::Vertical => {
                            self.layout_node(
                                child,
                                Constraint {
                                    min: glm::vec2(constraint.min.x, child_main),
                                    max: glm::vec2(constraint.max.x, child_main),
                                },
                            );
                            main += self.widgets[child].rect.extent.y;
                            cross = cross.max(self.widgets[child].rect.extent.x);
                            flex_available -= self.widgets[child].rect.extent.y;
                        }
                    }
                }

                child = self.widgets[child].next;
            }
        }

        match axis {
            Axis::Horizontal => {
                self.widgets[node].rect.extent = glm::vec2(
                    main + 2.0 * self.widgets[node].layout.padding,
                    cross + 2.0 * self.widgets[node].layout.padding,
                );
            }
            Axis::Vertical => {
                self.widgets[node].rect.extent = glm::vec2(
                    cross + 2.0 * self.widgets[node].layout.padding,
                    main + 2.0 * self.widgets[node].layout.padding,
                );
            }
        }
    }

    fn debug_render(&self, id: WidgetId, mut offset: Offset, gpu_data: &mut grr_2d::GpuData) {
        let widget = &self.widgets[id];
        let padding = glm::vec2(widget.layout.padding, widget.layout.padding);
        let rect_path = [grr_2d::Curve::Rect {
            p0: offset + padding,
            p1: offset + widget.rect.extent - padding,
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

        // dbg!((id, offset, widget.rect.extent));

        let axis = self.widgets[id].layout.axis;
        let mut child = self.widgets[id].first_child;
        while child != INVALID {
            // dbg!((id, child));
            self.debug_render(child, offset, gpu_data);
            match axis {
                Axis::Horizontal => offset.x += self.widgets[child].rect.extent.x,
                Axis::Vertical => offset.y += self.widgets[child].rect.extent.y,
            }
            child = self.widgets[child].next;
        }
    }
}

// Single Pass
struct RenderWidgets {
    num_widgets: usize,
}

impl RenderWidgets {
    fn new() -> Self {
        RenderWidgets { num_widgets: 0 }
    }

    fn reset(&mut self) {
        self.num_widgets = 0;
    }
}

struct Ui {
    phase: Option<Phase>,
    update: UpdateWidgets,
    layout: LayoutWidgets,
    render: RenderWidgets,
}

impl Ui {
    pub fn new() -> Self {
        Ui {
            phase: None,
            update: UpdateWidgets::new(),
            layout: LayoutWidgets::new(),
            render: RenderWidgets::new(),
        }
    }

    pub fn begin_phase(&mut self, phase: Phase) -> WidgetRef {
        assert!(self.phase.is_none());

        self.phase = Some(phase);

        let id = match phase {
            Phase::Update => {
                self.update.reset();
                let root = self.update.num_widgets;
                self.update.num_widgets += 1;
                root
            }
            Phase::Layout { .. } => {
                self.layout.reset();
                self.layout.add(Layout {
                    axis: Axis::Horizontal,
                    flex: 1.0,
                    extent: glm::vec2(0.0, 0.0),
                    padding: 0.0,
                })
            }
            Phase::Render => {
                self.render.reset();
                let root = self.update.num_widgets;
                self.update.num_widgets += 1;
                root
            }
        };

        WidgetRef { ui: self, id }
    }

    pub fn end_phase(&mut self) {
        match self.phase {
            Some(Phase::Layout { size }) => {
                self.layout.layout_node(
                    0,
                    Constraint {
                        min: glm::vec2(0.0, 0.0),
                        max: size,
                    },
                );
            }
            Some(Phase::Update) | Some(Phase::Render) => (),
            None => panic!("no phase started"),
        }

        self.phase = None;
    }

    fn add(&mut self, parent: WidgetId, widget: impl Widget) -> WidgetId {
        match self.phase {
            Some(Phase::Layout { .. }) => {
                let child = self.layout.add(widget.layout());
                let prev_child = self.layout.widgets[parent].last_child;
                self.layout.widgets[parent].last_child = child;

                match prev_child {
                    INVALID => {
                        self.layout.widgets[parent].first_child = child;
                    }
                    _ => {
                        self.layout.widgets[prev_child].next = child;
                    }
                }

                child
            }
            _ => self.layout.add(Layout {
                axis: Axis::Horizontal,
                flex: 0.0,
                extent: glm::vec2(0.0, 0.0),
                padding: 0.0,
            }), // dummy
        }
    }

    pub fn render_layout(&self, mut offset: Offset, gpu_data: &mut grr_2d::GpuData) {
        self.layout.debug_render(0, offset, gpu_data)
    }
}

struct WidgetRef<'a> {
    ui: &'a mut Ui,
    id: WidgetId,
}

impl WidgetRef<'_> {
    pub fn add(&mut self, widget: impl Widget) -> WidgetRef {
        let id = self.ui.add(self.id, widget);
        WidgetRef {
            ui: &mut self.ui,
            id,
        }
    }
}

pub enum EventPolicy {
    Pass,
    Acquire,
    Consume,
}

trait Widget {
    fn event(&self, event: &Event) -> EventPolicy;
    fn update(&self);
    fn layout(&self) -> Layout;
    fn render(&self);
}

#[derive(Debug, Copy, Clone)]
pub enum Axis {
    Horizontal, // x
    Vertical,   // y
}

impl Axis {
    fn main(&self, extent: Extent) -> f32 {
        match *self {
            Axis::Horizontal => extent.x,
            Axis::Vertical => extent.y,
        }
    }

    fn cross(&self, extent: Extent) -> f32 {
        match *self {
            Axis::Horizontal => extent.y,
            Axis::Vertical => extent.x,
        }
    }
}

struct Layout {
    axis: Axis,
    flex: f32,
    extent: Extent,
    padding: f32,
}

struct Constraint {
    pub min: Extent,
    pub max: Extent,
}

struct Rect {
    extent: Extent,
}

impl Rect {
    pub fn new() -> Self {
        Rect {
            extent: glm::vec2(0.0, 0.0),
        }
    }
}

pub enum Flex {
    Flex(Axis, f32),
    Static(Extent),
    Padding(f32),
}

impl Widget for Flex {
    fn event(&self, _: &Event) -> EventPolicy {
        EventPolicy::Pass
    }

    fn update(&self) {}

    fn layout(&self) -> Layout {
        match *self {
            Flex::Flex(axis, flex) => Layout {
                axis,
                flex,
                extent: glm::vec2(0.0, 0.0),
                padding: 0.0,
            },
            Flex::Static(extent) => Layout {
                axis: Axis::Horizontal,
                flex: 0.0,
                extent,
                padding: 0.0,
            },
            Flex::Padding(p) => Layout {
                axis: Axis::Horizontal,
                flex: 1.0,
                extent: glm::vec2(0.0, 0.0),
                padding: p,
            },
        }
    }

    fn render(&self) {}
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut ui = Ui::new();

    unsafe {
        grr_2d::run("layout", || {
            let mut gpu_data = grr_2d::GpuData::new();

            let mut root = ui.begin_phase(Phase::Layout {
                size: glm::vec2(400.0, 100.0),
            });
            let mut n0 = root.add(Flex::Flex(Axis::Horizontal, 1.0));
            n0.add(Flex::Static(glm::vec2(40.0, 20.0)));
            let mut n01 = n0.add(Flex::Flex(Axis::Vertical, 1.0));
            n01.add(Flex::Flex(Axis::Horizontal, 3.0));
            n01.add(Flex::Flex(Axis::Horizontal, 1.0));
            let mut n02 = n0.add(Flex::Flex(Axis::Horizontal, 1.0));
            n02.add(Flex::Padding(5.0));
            ui.end_phase();

            ui.render_layout(glm::vec2(0.0, 0.0), &mut gpu_data);

            gpu_data
        })
    }
}

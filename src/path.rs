use crate::glm;

pub type Segment = Vec<Curve>;

pub enum CurveCap {
    Butt,
    Round,
}

pub enum CurveJoin {
    Bevel,
    Round,
}

#[derive(Debug, Clone, Copy)]
pub enum Curve {
    Line {
        p0: glm::Vec2,
        p1: glm::Vec2,
    },
    Quad {
        p0: glm::Vec2,
        p1: glm::Vec2,
        p2: glm::Vec2,
    },
    Circle { center: glm::Vec2, radius: f32 },
    Arc { center: glm::Vec2, p0: glm::Vec2, p1: glm::Vec2 },
}

#[derive(Debug, Copy, Clone)]
pub struct Aabb {
    pub min: glm::Vec2,
    pub max: glm::Vec2,
}

impl Aabb {
    pub fn zero() -> Self {
        Aabb {
            min: glm::Vec2::new(0.0, 0.0),
            max: glm::Vec2::new(0.0, 0.0),
        }
    }

    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: glm::Vec2::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: glm::Vec2::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    pub fn from_curves(curves: &[Curve]) -> Self {
        curves
            .iter()
            .fold(Aabb {
                min: glm::vec2(std::f32::INFINITY, std::f32::INFINITY),
                max: glm::vec2(std::f32::NEG_INFINITY, std::f32::NEG_INFINITY),
            }, |aabb, curve| aabb.union(&curve.aabb()))
    }

    pub fn from_segments(segments: &[Segment]) -> Self {
        segments.iter().fold(Aabb::zero(), |aabb, segment| {
            aabb.union(&Aabb::from_curves(&segment))
        })
    }
}

impl Curve {
    pub fn aabb(&self) -> Aabb {
        match *self {
            Curve::Line { p0, p1 } => Aabb {
                min: glm::Vec2::new(p0.x.min(p1.x), p0.y.min(p1.y)),
                max: glm::Vec2::new(p0.x.max(p1.x), p0.y.max(p1.y)),
            },
            Curve::Quad { p0, p1, p2 } => {
                // bad approx - could be tighter
                Aabb {
                    min: glm::Vec2::new(p0.x.min(p1.x).min(p2.x), p0.y.min(p1.y).min(p2.y)),
                    max: glm::Vec2::new(p0.x.max(p1.x).max(p2.x), p0.y.max(p1.y).max(p2.y)),
                }
            }
            Curve::Circle { center, radius } => {
                dbg!(center + glm::vec2(radius, radius));
                Aabb {
                    min: center - glm::vec2(radius, radius),
                    max: center + glm::vec2(radius, radius),
                }
            }
            Curve::Arc { center, p0, p1 } => {
                Aabb {
                    min: glm::Vec2::new(p0.x.min(p1.x), p0.y.min(p1.y)),
                    max: glm::Vec2::new(p0.x.max(p1.x), p0.y.max(p1.y)),
                }
            }
        }
    }

    pub fn eval(&self, t: f32) -> glm::Vec2 {
        match *self {
            Curve::Line { p0, p1 } => (1.0 - t) * p0 + t * p1,
            Curve::Quad { p0, p1, p2 } => {
                (1.0 - t) * (1.0 - t) * p0 + 2.0 * t * (1.0 - t) * p1 + t * t * p2
            }
            Curve::Circle { .. } => todo!(),
            Curve::Arc { .. } => todo!(),
        }
    }

    pub fn monotonize(&self) -> Vec<Curve> {
        match *self {
            Curve::Line { .. } => vec![*self],
            Curve::Quad { p0, p1, p2 } => {
                let min = glm::vec2(p0.x.min(p2.x), p0.y.min(p2.y));
                let max = glm::vec2(p0.x.max(p2.x), p0.y.max(p2.y));

                let tx = if p1.x < min.x || max.x < p1.x {
                    Some((p0.x - p1.x) / (p0.x - 2.0 * p1.x + p2.x))
                } else {
                    None
                };

                let ty = if p1.y < min.y || max.y < p1.y {
                    Some((p0.y - p1.y) / (p0.y - 2.0 * p1.y + p2.y))
                } else {
                    None
                };

                match (tx, ty) {
                    (Some(tx), Some(ty)) => {
                        let t = tx.min(ty);
                        let p = self.eval(t);
                        let p10 = Curve::Line { p0, p1 }.eval(t);
                        let p11 = Curve::Line { p0: p1, p1: p2 }.eval(t);
                        let mut curves = vec![Curve::Quad { p0, p1: p10, p2: p }];
                        curves.extend(Curve::Quad { p0: p, p1: p11, p2 }.monotonize());
                        curves
                    }
                    (Some(t), None) | (None, Some(t)) => {
                        let p = self.eval(t);
                        let p10 = Curve::Line { p0, p1 }.eval(t);
                        let p11 = Curve::Line { p0: p1, p1: p2 }.eval(t);
                        vec![
                            Curve::Quad { p0, p1: p10, p2: p },
                            Curve::Quad { p0: p, p1: p11, p2 },
                        ]
                    }
                    (None, None) => vec![*self],
                }
            }
            Curve::Circle { .. } => vec![*self],
            Curve::Arc { .. } => vec![*self], // todo
        }
    }

    pub fn monotize_path(curves: &[Curve]) -> Vec<Curve> {
        curves
            .iter()
            .map(|curve| curve.monotonize())
            .flatten()
            .collect()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PathElement {
    MoveTo(glm::Vec2),
    LineTo(glm::Vec2),
    QuadTo(glm::Vec2, glm::Vec2),
    ArcTo(glm::Vec2, glm::Vec2),
    Close,
}

pub struct PathBuilder {
    elements: Vec<PathElement>,
}

impl PathBuilder {
    pub fn new() -> Self {
        PathBuilder {
            elements: Vec::new(),
        }
    }

    pub fn move_to(mut self, p: glm::Vec2) -> Self {
        self.elements.push(PathElement::MoveTo(p));
        self
    }

    pub fn line_to(mut self, p: glm::Vec2) -> Self {
        self.elements.push(PathElement::LineTo(p));
        self
    }

    pub fn quad_to(mut self, p1: glm::Vec2, p2: glm::Vec2) -> Self {
        self.elements.push(PathElement::QuadTo(p1, p2));
        self
    }

    pub fn arc_to(mut self, center: glm::Vec2, p1: glm::Vec2) -> Self {
        self.elements.push(PathElement::ArcTo(center, p1));
        self
    }

    pub fn close(mut self) -> Self {
        self.elements.push(PathElement::Close);
        self
    }

    pub fn monotonize(mut self) -> Self {
        let mut builder = PathBuilder::new();

        let mut p0 = glm::vec2(0.0f32, 0.0f32);
        let mut initial = p0;

        for element in self.elements {
            match element {
                PathElement::LineTo(p) => {
                    builder.elements.push(PathElement::LineTo(p));
                    p0 = p;
                }
                PathElement::MoveTo(p) => {
                    builder.elements.push(PathElement::MoveTo(p));
                    p0 = p;
                    initial = p;
                }
                PathElement::Close => {
                    builder.elements.push(PathElement::Close);
                    p0 = initial;
                }
                PathElement::LineTo(p) => {
                    builder.elements.push(PathElement::LineTo(p));
                    p0 = p;
                }
                PathElement::QuadTo(p1, p2) => {
                    let curves = Curve::Quad { p0, p1, p2}.monotonize();
                    for curve in curves {
                        if let Curve::Quad { p1, p2, .. } = curve {
                            builder.elements.push(PathElement::QuadTo(p1, p2));
                            p0 = p2;
                        } else {
                            unreachable!()
                        }
                    }
                }
                PathElement::ArcTo(_, _) => todo!(), // validation only
            }
        }

        builder
    }

    pub fn stroke(mut self, distance: f32, caps: (CurveCap, CurveJoin, CurveCap)) -> Vec<Curve> {
        let mut curves = Vec::new();

        let mut p0 = glm::vec2(0.0f32, 0.0f32);
        let mut n0 = glm::vec2(0.0f32, 0.0f32);
        let mut begin = None;

        let add_round_caps = |p: glm::Vec2, n: glm::Vec2, curves: &mut Vec<Curve>| {
            let pattern = [
                glm::vec2(-1.0, 0.0),
                glm::vec2(0.0, 1.0),
                glm::vec2(1.0, 0.0),
                glm::vec2(0.0, -1.0)
            ];
            let dirs = match (n.x <= 0.0, n.y <= 0.0) {
                (true, true) => 0,
                (true, false) => 1,
                (false, true) => 3,
                (false, false) => 2,
            };

            curves.push(Curve::Arc { center: p, p0: p + distance * n, p1: p + distance * pattern[dirs] });
            curves.push(Curve::Arc { center: p, p0: p + distance * pattern[dirs], p1: p + distance * pattern[(dirs + 1) % 4] });
            curves.push(Curve::Arc { center: p, p0: p + distance * pattern[(dirs + 1) % 4], p1: p - distance * n });
        };

        for element in &self.elements {
            match *element {
                PathElement::LineTo(p1) => {
                    let dir = glm::normalize(&(p1 - p0));
                    let n = glm::vec2(-dir.y, dir.x);

                    // extruded lines
                    curves.push(Curve::Line { p0: p0 + distance * n, p1: p1 + distance * n });
                    curves.push(Curve::Line { p0: p1 - distance * n, p1: p0 - distance * n });

                    match begin {
                        Some(_) => {
                            // direct connection to prior curve
                            curves.push(Curve::Line { p0: p0 + distance * n0, p1: p0 + distance * n });
                            curves.push(Curve::Line { p0: p0 - distance * n, p1: p0 - distance * n0 });

                            // arc caps
                            if let CurveJoin::Round = caps.1 {
                                curves.push(Curve::Circle { center: p0, radius: distance });
                            }
                        }
                        None => {
                            begin = Some((p0, n));
                        }
                    }

                    p0 = p1;
                    n0 = n;
                }
                PathElement::QuadTo(p1, p2) => {
                    let d0 = glm::normalize(&(p1 - p0));
                    let d2 = glm::normalize(&(p2 - p1));
                    let normal0 = glm::vec2(-d0.y, d0.x);
                    let normal2 = glm::vec2(-d2.y, d2.x);
                    let normal1 = (normal0 + normal2) / (1.0 + glm::dot(&normal0, &normal2));

                    // extruded lines
                    curves.push(Curve::Quad {
                        p0: p0 + distance * normal0,
                        p1: p1 + distance * normal1,
                        p2: p2 + distance * normal2,
                    });
                    curves.push(Curve::Quad {
                        p0: p2 - distance * normal2,
                        p1: p1 - distance * normal1,
                        p2: p0 - distance * normal0,
                    });

                    match begin {
                        Some(_) => {
                            // direct connection to prior curve
                            curves.push(Curve::Line { p0: p0 + distance * n0, p1: p0 + distance * normal0 });
                            curves.push(Curve::Line { p0: p0 - distance * normal0, p1: p0 - distance * n0 });

                            // arc caps
                            if let CurveJoin::Round = caps.1 {
                                curves.push(Curve::Circle { center: p0, radius: distance });
                            }
                        }
                        None => {
                            begin = Some((p0, normal0));
                        }
                    }

                    p0 = p2;
                    n0 = normal2;
                }
                PathElement::MoveTo(p) => {
                    if let Some((p, n)) = begin.take() {
                        // close off
                        match caps.0 {
                            CurveCap::Round => add_round_caps(p, -n, &mut curves),
                            CurveCap::Butt => { curves.push(Curve::Line { p0: p - distance * n, p1: p + distance * n }); }
                        }

                        match caps.2 {
                            CurveCap::Round => add_round_caps(p0, n0, &mut curves),
                            CurveCap::Butt => { curves.push(Curve::Line { p0: p0 + distance * n0, p1: p0 - distance * n0 }); }
                        }
                    }

                    p0 = p;
                }
                PathElement::Close => {
                    if let Some((p1, n1)) = begin.take() {
                        let dir = glm::normalize(&(p1 - p0));
                        let n = glm::vec2(-dir.y, dir.x);

                        curves.push(Curve::Line { p0: p0 + distance * n0, p1: p0 + distance * n }); // connection to prior
                        curves.push(Curve::Line { p0: p0 + distance * n, p1: p1 + distance * n }); // extruded
                        curves.push(Curve::Line { p0: p1 + distance * n, p1: p1 + distance * n1 }); // connection to initial

                        curves.push(Curve::Line { p0: p0 - distance * n, p1: p0 - distance * n0 }); // connection to prior
                        curves.push(Curve::Line { p0: p1 - distance * n, p1: p0 - distance * n }); // extruded
                        curves.push(Curve::Line { p0: p1 - distance * n1, p1: p1 - distance * n }); // connection to initial

                        if let CurveJoin::Round = caps.1 {
                            curves.push(Curve::Circle { center: p0, radius: distance }); // arc cap prior
                            curves.push(Curve::Circle { center: p1, radius: distance }); // arc cap initial
                        }
                    }

                }
                PathElement::ArcTo(_, _) => todo!(),
            }
        }

        // remaining path - same as a move to
        if let Some((p, n)) = begin.take() {
            // close off
            match caps.0 {
                CurveCap::Round => add_round_caps(p, -n, &mut curves),
                CurveCap::Butt => { curves.push(Curve::Line { p0: p - distance * n, p1: p + distance * n }); }
            }

            match caps.2 {
                CurveCap::Round => add_round_caps(p0, n0, &mut curves),
                CurveCap::Butt => { curves.push(Curve::Line { p0: p0 + distance * n0, p1: p0 - distance * n0 }); }
            }
        }

        curves
    }

    pub fn fill(self) -> PathSplitter {
        let mut splitter = PathSplitter::new();
        for element in self.elements {
            splitter = match element {
                PathElement::MoveTo(p) => splitter.move_to(p),
                PathElement::LineTo(p) => splitter.line_to(p),
                PathElement::QuadTo(p1, p2) => splitter.quad_to(p1, p2),
                PathElement::ArcTo(center, p1) => splitter.arc_to(center, p1),
                PathElement::Close => splitter.close(),
            };
        }
        splitter
    }
}

pub struct PathSplitter {
    curves: Vec<Curve>,
    first: glm::Vec2,
    last: glm::Vec2,
}

impl PathSplitter {
    pub fn new() -> Self {
        PathSplitter {
            curves: Vec::new(),
            first: glm::vec2(0.0, 0.0),
            last: glm::vec2(0.0, 0.0),
        }
    }

    pub fn move_to(mut self, p: glm::Vec2) -> Self {
        self.first = p;
        self.last = p;
        self
    }

    pub fn line_to(mut self, p: glm::Vec2) -> Self {
        self.curves.push(Curve::Line {
            p0: self.last,
            p1: p,
        });
        self.last = p;
        self
    }

    pub fn quad_to(mut self, p1: glm::Vec2, p2: glm::Vec2) -> Self {
        self.curves.push(Curve::Quad {
            p0: self.last,
            p1,
            p2,
        });
        self.last = p2;
        self
    }

    // spooky
    fn arc_to(mut self, center: glm::Vec2, p1: glm::Vec2) -> Self {
        self.curves.push(Curve::Arc {
            center,
            p0: self.last,
            p1,
        });
        self.last = p1;
        self
    }

    pub fn close(mut self) -> Self {
        self.curves.push(Curve::Line {
            p0: self.last,
            p1: self.first,
        });
        self.last = self.first;
        self
    }

    pub fn finish(self) -> Vec<Curve> {
        self.curves
    }
}

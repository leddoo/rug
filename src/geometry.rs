#[derive(Debug, Clone, Copy, PartialEq)]
pub struct V2f {
    pub x: f32,
    pub y: f32,
}

pub fn v2f(x: f32, y: f32) -> V2f {
    V2f { x, y }
}

impl Into<(f32, f32)> for V2f {
    fn into(self) -> (f32, f32) {
        (self.x, self.y)
    }
}

impl std::ops::Neg for V2f {
    type Output = V2f;

    fn neg(self) -> V2f {
        V2f {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl std::ops::Add for V2f {
    type Output = V2f;

    fn add(self, other: V2f) -> V2f {
        V2f {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Sub for V2f {
    type Output = V2f;

    fn sub(self, other: V2f) -> V2f {
        V2f {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Mul<V2f> for V2f {
    type Output = V2f;

    fn mul(self, other: V2f) -> V2f {
        V2f {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl std::ops::Div<V2f> for V2f {
    type Output = V2f;

    fn div(self, other: V2f) -> V2f {
        V2f {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

impl std::ops::Mul<V2f> for f32 {
    type Output = V2f;

    fn mul(self, vec: V2f) -> V2f {
        V2f {
            x: self * vec.x,
            y: self * vec.y,
        }
    }
}

impl std::ops::Div<f32> for V2f {
    type Output = V2f;

    fn div(self, scalar: f32) -> V2f {
        V2f {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl V2f {
    pub fn dot(self, other: V2f) -> f32 {
        (self.x * other.x) + (self.y * other.y)
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn lerp(self, other: Self, t: f32) -> V2f {
        (1.0 - t)*self + t*other
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub p0: V2f,
    pub p1: V2f,
}

pub fn segment(p0: V2f, p1: V2f) -> Segment {
    Segment { p0, p1 }
}

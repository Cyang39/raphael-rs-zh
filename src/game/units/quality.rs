#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quality {
    value: u32,
}

impl Quality {
    pub const fn scale(self, mul: u32, div: u32) -> Self {
        Self {
            value: self.value * mul / div,
        }
    }

    pub const fn from_const(value: u32) -> Self {
        Self { value: value * 800 }
    }

    pub const fn is_zero(self) -> bool {
        self.value == 0
    }
}

impl std::convert::From<u32> for Quality {
    fn from(value: u32) -> Self {
        Self { value: value * 800 }
    }
}

impl std::convert::From<f32> for Quality {
    fn from(value: f32) -> Self {
        Self { value: (value * 800.0).ceil() as u32 }
    }
}

impl std::convert::From<Quality> for f32 {
    fn from(value: Quality) -> Self {
        value.value as f32 / 800.0
    }
}

impl std::ops::Add for Quality {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            value: self.value + other.value,
        }
    }
}

impl std::ops::AddAssign for Quality {
    fn add_assign(&mut self, rhs: Self) {
        self.value += rhs.value;
    }
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let result: f32 = (*self).into();
        write!(f, "{:.2}", result)
    }
}
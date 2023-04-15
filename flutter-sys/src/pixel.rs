#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Pixel {
    pub fn zero() -> Self {
        Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
}

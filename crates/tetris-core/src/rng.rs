#[derive(Debug, Clone)]
pub struct Lcg(pub u32);

impl Lcg {
    pub fn new(seed: u32) -> Self {
        Lcg(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }
}

// smoother for pitch CART model (in _duration.rs)
pub struct model { pitch: f32, alpha: f32, yp1: f32, }
impl model {
	pub fn new() -> model { model { pitch: 50.0, alpha: 0.999, yp1: 50.0, } }
	pub fn set(&mut self, x: f32) { self.pitch = x; }
	pub fn reset(&mut self) {
		self.pitch = 50.0;
		self.yp1   = 50.0;
	}
	pub fn step(&mut self) -> f32 {
		// some good ol' smoothing action
		let y: f32 = (1.0 - self.alpha) * self.pitch + self.alpha * self.yp1;
		self.yp1 = y;
		y
	}
}

#[derive(Clone, Copy)]
pub struct Spring {
    pub value: f32,
    velocity: f32,
    pub target: f32,
}

impl Spring {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.,
            target: value,
        }
    }

    pub fn step(&mut self, dt: f32, stiffness: f32, damping: f32) -> bool {
        let steps = (dt / 0.004).ceil().max(1.) as usize;
        let step = dt / steps as f32;
        for _ in 0..steps {
            self.velocity +=
                (stiffness * (self.target - self.value) - damping * self.velocity) * step;
            self.value += self.velocity * step;
        }
        let moving = (self.target - self.value).abs() > 0.002 || self.velocity.abs() > 0.01;
        if !moving {
            self.value = self.target;
            self.velocity = 0.;
        }
        moving
    }
}

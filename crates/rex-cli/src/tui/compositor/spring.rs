//! RK4 damped spring for modal and banner motion.

#[derive(Debug, Clone)]
pub struct SpringState {
    position: f32,
    velocity: f32,
    target: f32,
    stiffness: f32,
    damping: f32,
    settled_eps: f32,
}

impl SpringState {
    pub fn modal_entrance() -> Self {
        Self {
            position: 1.0,
            velocity: 0.0,
            target: 0.0,
            stiffness: 180.0,
            damping: 22.0,
            settled_eps: 0.002,
        }
    }

    pub fn banner_drop() -> Self {
        Self {
            position: -1.0,
            velocity: 0.0,
            target: 0.0,
            stiffness: 220.0,
            damping: 24.0,
            settled_eps: 0.002,
        }
    }

    pub fn reset_entrance(&mut self) {
        self.position = 1.0;
        self.velocity = 0.0;
    }

    pub fn active(&self) -> bool {
        (self.position - self.target).abs() > self.settled_eps
            || self.velocity.abs() > self.settled_eps
    }

    pub fn settled(&self) -> bool {
        !self.active()
    }

    pub fn offset_rows(&self) -> i16 {
        (self.position * 4.0).round() as i16
    }

    pub fn step(&mut self) {
        if !self.active() {
            return;
        }
        let dt = 1.0 / 60.0;
        let mut pos = self.position;
        let mut vel = self.velocity;
        for _ in 0..4 {
            let force = -self.stiffness * (pos - self.target) - self.damping * vel;
            let a = force;
            let k1_v = a * dt;
            let k1_p = vel * dt;
            let k2_v = a * dt;
            let k2_p = (vel + k1_v * 0.5) * dt;
            let k3_v = a * dt;
            let k3_p = (vel + k2_v * 0.5) * dt;
            let k4_v = a * dt;
            let k4_p = (vel + k3_v) * dt;
            vel += (k1_v + 2.0 * k2_v + 2.0 * k3_v + k4_v) / 6.0;
            pos += (k1_p + 2.0 * k2_p + 2.0 * k3_p + k4_p) / 6.0;
        }
        self.position = pos;
        self.velocity = vel;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_settles_toward_target() {
        let mut s = SpringState::modal_entrance();
        for _ in 0..120 {
            s.step();
        }
        assert!(s.settled());
        assert!(s.offset_rows().abs() <= 1);
    }
}

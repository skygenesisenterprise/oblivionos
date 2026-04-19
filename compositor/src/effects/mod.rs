use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlurEffect {
    None,
    Behind,
    Vibe,
}

impl Default for BlurEffect {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowStyle {
    pub enabled: bool,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub opacity: f32,
}

impl Default for ShadowStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            offset_x: 0.0,
            offset_y: 4.0,
            blur_radius: 12.0,
            spread_radius: 0.0,
            opacity: 0.25,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationConfig {
    pub enabled: bool,
    pub duration_ms: u32,
    pub easing: EasingFunction,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_ms: 200,
            easing: EasingFunction::EaseOut,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
    Spring { mass: f32, stiffness: f32, damping: f32 },
}

impl EasingFunction {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOut => {
                if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
            }
            EasingFunction::CubicBezier(x1, y1, x2, y2) => {
                Self::cubic_bezier(*x1, *y1, *x2, *y2, t)
            }
            EasingFunction::Spring { mass, stiffness, damping } => {
                Self::spring(*mass, *stiffness, *damping, t)
            }
        }
    }

    fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
        let cx = 3.0 * x1;
        let bx = 3.0 * (x2 - x1) - cx;
        let ax = 1.0 - cx - bx;
        let cy = 3.0 * y1;
        let by = 3.0 * (y2 - y1) - cy;
        let ay = 1.0 - cy - by;
        let x_val = ((ax * t + bx) * t + cx) * t;
        ((ay * t + by) * t + cy) * t
    }

    fn spring(mass: f32, stiffness: f32, damping: f32, t: f32) -> f32 {
        let omega = (stiffness / mass).sqrt();
        let zeta = damping / (2.0 * (mass * stiffness).sqrt());
        if zeta < 1.0 {
            let omega_d = omega * (1.0 - zeta * zeta).sqrt();
            (-zeta * omega * t).exp() * ((zeta * omega / omega_d).sin() * t.cos() + t.sin() * omega_d.sin())
        } else {
            (1.0 + omega * t).exp().recip()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionType {
    None,
    Fade,
    Slide,
    Scale,
    FadeAndSlide,
    FadeAndScale,
}

impl Default for TransitionType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone)]
pub struct WindowEffects {
    pub blur: BlurEffect,
    pub shadow: ShadowStyle,
    pub animations: AnimationConfig,
    pub transition: TransitionType,
    pub brightness: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub corner_radius: (u32, u32, u32, u32),
}

impl Default for WindowEffects {
    fn default() -> Self {
        Self {
            blur: BlurEffect::None,
            shadow: ShadowStyle::default(),
            animations: AnimationConfig::default(),
            transition: TransitionType::None,
            brightness: 1.0,
            saturation: 1.0,
            contrast: 1.0,
            corner_radius: (12, 12, 12, 12),
        }
    }
}

impl WindowEffects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_blur(mut self, blur: BlurEffect) -> Self {
        self.blur = blur;
        self
    }

    pub fn with_shadow(mut self, shadow: ShadowStyle) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn with_corner_radius(mut self, radius: (u32, u32, u32, u32)) -> Self {
        self.corner_radius = radius;
        self
    }

    pub fn set_blur(&mut self, blur: BlurEffect) {
        self.blur = blur;
    }

    pub fn set_shadow(&mut self, enabled: bool) {
        self.shadow.enabled = enabled;
    }
}

pub struct Animation {
    pub start_value: f32,
    pub end_value: f32,
    pub current_value: f32,
    pub duration: Duration,
    pub elapsed: Duration,
    pub easing: EasingFunction,
    pub running: bool,
}

impl Animation {
    pub fn new(start: f32, end: f32, duration: Duration) -> Self {
        Self {
            start_value: start,
            end_value: end,
            current_value: start,
            duration,
            elapsed: Duration::ZERO,
            easing: EasingFunction::EaseOut,
            running: true,
        }
    }

    pub fn update(&mut self, delta: Duration) -> bool {
        if !self.running {
            return false;
        }

        self.elapsed += delta;
        let progress = (self.elapsed.as_millis() as f32 / self.duration.as_millis() as f32).clamp(0.0, 1.0);
        
        let eased = self.easing.apply(progress);
        self.current_value = self.start_value + (self.end_value - self.start_value) * eased;

        if progress >= 1.0 {
            self.running = false;
            self.current_value = self.end_value;
        }

        !self.running
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn reset(&mut self) {
        self.current_value = self.start_value;
        self.elapsed = Duration::ZERO;
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}
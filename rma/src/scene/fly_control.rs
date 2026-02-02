use cgmath::prelude::*;
use three_d::*;

/// Camera state - stores mode-appropriate data
#[derive(Clone)]
pub enum CameraState {
    Orbit {
        target: Vec3,
        distance: f32,
        yaw: f32,
        pitch: f32,
    },
    Fly {
        position: Vec3,
        yaw: f32,
        pitch: f32,
    },
}

impl CameraState {
    pub fn new_orbit(target: Vec3, distance: f32, yaw: f32, pitch: f32) -> Self {
        Self::Orbit {
            target,
            distance,
            yaw,
            pitch: pitch.clamp(-1.5, 1.5),
        }
    }

    pub fn new_fly(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self::Fly {
            position,
            yaw,
            pitch: pitch.clamp(-1.5, 1.5),
        }
    }

    /// Convert to orbit mode, preserving current view
    pub fn to_orbit(&self, default_distance: f32) -> Self {
        match self {
            Self::Orbit { .. } => self.clone(),
            Self::Fly {
                position,
                yaw,
                pitch,
            } => {
                let forward = Self::forward_from_angles(*yaw, *pitch);
                let target = *position + forward * default_distance;
                Self::Orbit {
                    target,
                    distance: default_distance,
                    yaw: *yaw,
                    pitch: *pitch,
                }
            }
        }
    }

    /// Convert to fly mode, preserving current view
    pub fn to_fly(&self) -> Self {
        match self {
            Self::Fly { .. } => self.clone(),
            Self::Orbit {
                target,
                distance,
                yaw,
                pitch,
            } => {
                let position = *target - Self::forward_from_angles(*yaw, *pitch) * *distance;
                Self::Fly {
                    position,
                    yaw: *yaw,
                    pitch: *pitch,
                }
            }
        }
    }

    pub fn is_orbit(&self) -> bool {
        matches!(self, Self::Orbit { .. })
    }

    pub fn is_fly(&self) -> bool {
        matches!(self, Self::Fly { .. })
    }

    fn forward_from_angles(yaw: f32, pitch: f32) -> Vec3 {
        vec3(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        )
    }

    pub fn position(&self) -> Vec3 {
        match self {
            Self::Orbit {
                target,
                distance,
                yaw,
                pitch,
            } => {
                // Position is behind the target along the view direction
                *target - Self::forward_from_angles(*yaw, *pitch) * *distance
            }
            Self::Fly { position, .. } => *position,
        }
    }

    pub fn forward(&self) -> Vec3 {
        match self {
            Self::Orbit { yaw, pitch, .. } | Self::Fly { yaw, pitch, .. } => {
                Self::forward_from_angles(*yaw, *pitch)
            }
        }
    }

    pub fn right(&self) -> Vec3 {
        let up = vec3(0.0, 0.0, 1.0);
        self.forward().cross(up).normalize()
    }

    pub fn target(&self) -> Vec3 {
        match self {
            Self::Orbit { target, .. } => *target,
            Self::Fly {
                position,
                yaw,
                pitch,
            } => *position + Self::forward_from_angles(*yaw, *pitch),
        }
    }
}

/// Camera with mode-based state and projection parameters.
pub struct Camera {
    pub state: CameraState,
    viewport: Viewport,
    fov_y: Radians,
    z_near: f32,
    z_far: f32,
    pub tone_mapping: ToneMapping,
    pub color_mapping: ColorMapping,
}

impl Camera {
    pub fn new_orbit(
        viewport: Viewport,
        target: Vec3,
        distance: f32,
        yaw: f32,
        pitch: f32,
        field_of_view_y: impl Into<Radians>,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            state: CameraState::new_orbit(target, distance, yaw, pitch),
            viewport,
            fov_y: field_of_view_y.into(),
            z_near,
            z_far,
            tone_mapping: ToneMapping::default(),
            color_mapping: ColorMapping::default(),
        }
    }

    pub fn new_fly(
        viewport: Viewport,
        position: Vec3,
        yaw: f32,
        pitch: f32,
        field_of_view_y: impl Into<Radians>,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            state: CameraState::new_fly(position, yaw, pitch),
            viewport,
            fov_y: field_of_view_y.into(),
            z_near,
            z_far,
            tone_mapping: ToneMapping::default(),
            color_mapping: ColorMapping::default(),
        }
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }
}

impl Viewer for Camera {
    fn position(&self) -> Vec3 {
        self.state.position()
    }

    fn view(&self) -> Mat4 {
        let position = self.state.position();
        let target = position + self.state.forward();
        let up = vec3(0.0, 0.0, 1.0);
        Mat4::look_at_rh(Point3::from_vec(position), Point3::from_vec(target), up)
    }

    fn projection(&self) -> Mat4 {
        cgmath::perspective(self.fov_y, self.viewport.aspect(), self.z_near, self.z_far)
    }

    fn viewport(&self) -> Viewport {
        self.viewport
    }

    fn z_near(&self) -> f32 {
        self.z_near
    }

    fn z_far(&self) -> f32 {
        self.z_far
    }

    fn color_mapping(&self) -> ColorMapping {
        self.color_mapping
    }

    fn tone_mapping(&self) -> ToneMapping {
        self.tone_mapping
    }
}

/// Unified camera control for both orbit and fly modes
pub struct CameraControl {
    // Orbit settings
    pub min_distance: f32,
    pub max_distance: f32,

    // Fly settings
    pub fly_speed: f32,

    // Shared settings
    pub sensitivity: f32,

    // Mouse state
    pub left_pressed: bool,
    pub right_pressed: bool,

    // Movement state (fly mode)
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
}

impl CameraControl {
    pub fn new(min_distance: f32, max_distance: f32) -> Self {
        Self {
            min_distance,
            max_distance,
            fly_speed: 10.0,
            sensitivity: 0.001,
            left_pressed: false,
            right_pressed: false,
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
        }
    }

    pub fn handle_events(&mut self, state: &mut CameraState, events: &mut [Event], dt: f32) {
        for event in events.iter_mut() {
            match event {
                Event::MousePress {
                    button, handled, ..
                } => {
                    if !*handled {
                        match button {
                            MouseButton::Left => self.left_pressed = true,
                            MouseButton::Right => self.right_pressed = true,
                            _ => continue,
                        }
                        *handled = true;
                    }
                }
                Event::MouseRelease {
                    button, handled, ..
                } => {
                    match button {
                        MouseButton::Left => self.left_pressed = false,
                        MouseButton::Right => self.right_pressed = false,
                        _ => continue,
                    }
                    if !*handled {
                        *handled = true;
                    }
                }
                Event::RawMouseMotion { delta, handled, .. } => {
                    if self.left_pressed {
                        self.handle_look(state, *delta);
                        if !*handled {
                            *handled = true;
                        }
                    } else if self.right_pressed && state.is_orbit() {
                        self.handle_pan(state, *delta);
                        if !*handled {
                            *handled = true;
                        }
                    }
                }
                Event::MouseWheel { delta, handled, .. } => {
                    if !*handled {
                        self.handle_scroll(state, delta.1);
                        *handled = true;
                    }
                }
                Event::KeyPress { kind, handled, .. } if state.is_fly() => {
                    if !*handled && self.handle_key(*kind, true) {
                        *handled = true;
                    }
                }
                Event::KeyRelease { kind, handled, .. } if state.is_fly() => {
                    if !*handled && self.handle_key(*kind, false) {
                        *handled = true;
                    }
                }
                Event::ModifiersChange { modifiers } if state.is_fly() => {
                    self.move_down = modifiers.shift;
                }
                _ => {}
            }
        }

        // Apply fly movement
        if let CameraState::Fly {
            position,
            yaw,
            pitch,
        } = state
        {
            self.apply_movement(position, *yaw, *pitch, dt);
        }
    }

    fn handle_look(&self, state: &mut CameraState, delta: (f64, f64)) {
        let (dx, dy) = (delta.0 as f32, delta.1 as f32);
        match state {
            CameraState::Orbit { yaw, pitch, .. } | CameraState::Fly { yaw, pitch, .. } => {
                *yaw -= dx * self.sensitivity;
                *pitch = (*pitch - dy * self.sensitivity).clamp(-1.5, 1.5);
            }
        }
    }

    fn handle_pan(&self, state: &mut CameraState, delta: (f64, f64)) {
        let CameraState::Orbit {
            target, yaw, pitch, ..
        } = state
        else {
            return;
        };
        let (dx, dy) = (delta.0 as f32, delta.1 as f32);
        let speed = 0.5;
        let forward = CameraState::forward_from_angles(*yaw, *pitch);
        let up = vec3(0.0, 0.0, 1.0);
        let right = forward.cross(up).normalize();
        *target += right * (dx * speed) + up * (-dy * speed);
    }

    fn handle_scroll(&mut self, state: &mut CameraState, delta_y: f32) {
        match state {
            CameraState::Orbit { distance, .. } => {
                *distance = (*distance * (-delta_y * 0.01).exp())
                    .clamp(self.min_distance, self.max_distance);
            }
            CameraState::Fly { .. } => {
                self.fly_speed *= (delta_y * 0.01).exp();
            }
        }
    }

    fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        match key {
            Key::W => self.move_forward = pressed,
            Key::S => self.move_backward = pressed,
            Key::A => self.move_left = pressed,
            Key::D => self.move_right = pressed,
            Key::Space => self.move_up = pressed,
            _ => return false,
        }
        true
    }

    fn apply_movement(&self, position: &mut Vec3, yaw: f32, pitch: f32, dt: f32) {
        let forward = CameraState::forward_from_angles(yaw, pitch);
        let up = vec3(0.0, 0.0, 1.0);
        let left = forward.cross(up).normalize();

        let mut movement = vec3(0.0, 0.0, 0.0);
        if self.move_forward {
            movement += forward;
        }
        if self.move_backward {
            movement -= forward;
        }
        if self.move_right {
            movement += left;
        }
        if self.move_left {
            movement -= left;
        }
        if self.move_up {
            movement += up;
        }
        if self.move_down {
            movement -= up;
        }

        if movement.magnitude() > 0.0 {
            *position += movement.normalize() * self.fly_speed * dt;
        }
    }
}

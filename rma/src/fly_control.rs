use three_d::*;

/// Fly camera control with WASD + Q/E movement
pub struct FlyControl {
    speed: f32,
    /// Keys currently held down
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    /// Mouse look state
    is_looking: bool,
    last_mouse_pos: Option<(f32, f32)>,
}

impl Default for FlyControl {
    fn default() -> Self {
        Self {
            speed: 50.0,
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            is_looking: false,
            last_mouse_pos: None,
        }
    }
}

impl FlyControl {
    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn handle_events(&mut self, camera: &mut Camera, events: &mut [Event], dt: f32) {
        for event in events.iter_mut() {
            match event {
                Event::KeyPress { kind, handled, .. } => {
                    if !*handled {
                        let consumed = match kind {
                            Key::W => {
                                self.forward = true;
                                true
                            }
                            Key::S => {
                                self.backward = true;
                                true
                            }
                            Key::A => {
                                self.left = true;
                                true
                            }
                            Key::D => {
                                self.right = true;
                                true
                            }
                            Key::Q => {
                                self.down = true;
                                true
                            }
                            Key::E => {
                                self.up = true;
                                true
                            }
                            _ => false,
                        };
                        if consumed {
                            *handled = true;
                        }
                    }
                }
                Event::KeyRelease { kind, handled, .. } => {
                    if !*handled {
                        let consumed = match kind {
                            Key::W => {
                                self.forward = false;
                                true
                            }
                            Key::S => {
                                self.backward = false;
                                true
                            }
                            Key::A => {
                                self.left = false;
                                true
                            }
                            Key::D => {
                                self.right = false;
                                true
                            }
                            Key::Q => {
                                self.down = false;
                                true
                            }
                            Key::E => {
                                self.up = false;
                                true
                            }
                            _ => false,
                        };
                        if consumed {
                            *handled = true;
                        }
                    }
                }
                Event::MouseWheel { delta, handled, .. } => {
                    if !*handled {
                        // Adjust speed with scroll wheel
                        self.speed *= (delta.1 * 0.01).exp();
                        *handled = true;
                    }
                }
                Event::MousePress {
                    button,
                    position,
                    handled,
                    ..
                } => {
                    if !*handled && *button == MouseButton::Left {
                        self.is_looking = true;
                        self.last_mouse_pos = Some((position.x, position.y));
                        *handled = true;
                    }
                }
                Event::MouseRelease {
                    button, handled, ..
                } => {
                    if *button == MouseButton::Left {
                        self.is_looking = false;
                        self.last_mouse_pos = None;
                        if !*handled {
                            *handled = true;
                        }
                    }
                }
                Event::MouseMotion {
                    position, handled, ..
                } => {
                    if self.is_looking {
                        if let Some((last_x, last_y)) = self.last_mouse_pos {
                            let dx = position.x - last_x;
                            let dy = position.y - last_y;

                            // Rotate camera (yaw around world up, pitch around camera right)
                            let sensitivity = 0.002;

                            let pos = camera.position();
                            let target = camera.target();
                            let up = vec3(0.0, 0.0, 1.0); // World up

                            let forward = (target - pos).normalize();
                            let right = forward.cross(up).normalize();

                            // Yaw rotation (around world up axis)
                            let yaw = Mat4::from_axis_angle(up, radians(-dx * sensitivity));
                            // Pitch rotation (around camera right axis)
                            let pitch = Mat4::from_axis_angle(right, radians(dy * sensitivity));

                            // Apply rotations to forward vector
                            let forward4 = vec4(forward.x, forward.y, forward.z, 0.0);
                            let rotated = pitch * yaw * forward4;
                            let new_forward = vec3(rotated.x, rotated.y, rotated.z).normalize();

                            // Prevent flipping by clamping pitch
                            if new_forward.z.abs() < 0.99 {
                                let new_target = pos + new_forward;
                                camera.set_view(pos, new_target, up);
                            }
                        }
                        self.last_mouse_pos = Some((position.x, position.y));
                        if !*handled {
                            *handled = true;
                        }
                    }
                }
                _ => {}
            }
        }

        // Apply movement based on held keys
        let pos = camera.position();
        let target = camera.target();
        let up = vec3(0.0, 0.0, 1.0);

        let forward = (target - pos).normalize();
        let right = forward.cross(up).normalize();

        let mut movement = vec3(0.0, 0.0, 0.0);

        if self.forward {
            movement += forward;
        }
        if self.backward {
            movement -= forward;
        }
        if self.right {
            movement += right;
        }
        if self.left {
            movement -= right;
        }
        if self.up {
            movement += up;
        }
        if self.down {
            movement -= up;
        }

        if movement.magnitude() > 0.0 {
            movement = movement.normalize() * self.speed * dt;
            let new_pos = pos + movement;
            let new_target = target + movement;
            camera.set_view(new_pos, new_target, up);
        }
    }
}

use crate::app::AppState;
use nalgebra_glm as glm;
use winit::event::VirtualKeyCode;

pub enum CameraDirection {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

pub struct FreeCamera {
    pub position: glm::Vec3,
    right: glm::Vec3,
    pub front: glm::Vec3,
    up: glm::Vec3,
    world_up: glm::Vec3,
    speed: f32,
    sensitivity: f32,
    yaw_degrees: f32,
    pitch_degrees: f32,
}

impl Default for FreeCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl FreeCamera {
    pub fn new() -> Self {
        let mut camera = Self {
            position: glm::vec3(0.0, 0.0, 10.0),
            right: glm::vec3(0.0, 0.0, 0.0),
            front: glm::vec3(0.0, 0.0, -1.0),
            up: glm::vec3(0.0, 0.0, 0.0),
            world_up: glm::vec3(0.0, 1.0, 0.0),
            speed: 20.0,
            sensitivity: 0.05,
            yaw_degrees: -90.0,
            pitch_degrees: 0.0,
        };
        camera.calculate_vectors();
        camera
    }

    pub fn position_at(&mut self, position: &glm::Vec3) {
        self.position = *position;
        self.calculate_vectors();
    }

    pub fn look_at(&mut self, target: &glm::Vec3) {
        self.front = (target - self.position).normalize();
        self.pitch_degrees = self.front.y.asin().to_degrees();
        self.yaw_degrees = (self.front.x / self.front.y.asin().cos())
            .acos()
            .to_degrees();
        self.calculate_vectors();
    }

    pub fn view_matrix(&self) -> glm::Mat4 {
        let target = self.position + self.front;
        glm::look_at(&self.position, &target, &self.up)
    }

    fn translate(&mut self, direction: CameraDirection, delta_time: f32) {
        let velocity = self.speed * delta_time;
        match direction {
            CameraDirection::Forward => self.position += self.front * velocity,
            CameraDirection::Backward => self.position -= self.front * velocity,
            CameraDirection::Left => self.position -= self.right * velocity,
            CameraDirection::Right => self.position += self.right * velocity,
            CameraDirection::Up => self.position -= self.up * velocity,
            CameraDirection::Down => self.position += self.up * velocity,
        };
    }

    fn process_mouse_movement(&mut self, x_offset: f32, y_offset: f32) {
        let (x_offset, y_offset) = (x_offset * self.sensitivity, y_offset * self.sensitivity);

        self.yaw_degrees -= x_offset;
        self.pitch_degrees -= y_offset;

        let pitch_threshold = 89.0;
        if self.pitch_degrees > pitch_threshold {
            self.pitch_degrees = pitch_threshold
        } else if self.pitch_degrees < -pitch_threshold {
            self.pitch_degrees = -pitch_threshold
        }

        self.calculate_vectors();
    }

    fn calculate_vectors(&mut self) {
        let pitch_radians = self.pitch_degrees.to_radians();
        let yaw_radians = self.yaw_degrees.to_radians();
        self.front = glm::vec3(
            pitch_radians.cos() * yaw_radians.cos(),
            pitch_radians.sin(),
            yaw_radians.sin() * pitch_radians.cos(),
        )
        .normalize();
        self.right = self.front.cross(&self.world_up).normalize();
        self.up = self.right.cross(&self.front).normalize();
    }

    pub fn update(&mut self, app_state: &AppState) {
        if app_state.input.is_key_pressed(VirtualKeyCode::W) {
            self.translate(CameraDirection::Forward, app_state.delta_time as f32);
        }

        if app_state.input.is_key_pressed(VirtualKeyCode::A) {
            self.translate(CameraDirection::Left, app_state.delta_time as f32);
        }

        if app_state.input.is_key_pressed(VirtualKeyCode::S) {
            self.translate(CameraDirection::Backward, app_state.delta_time as f32);
        }

        if app_state.input.is_key_pressed(VirtualKeyCode::D) {
            self.translate(CameraDirection::Right, app_state.delta_time as f32);
        }

        if app_state.input.is_key_pressed(VirtualKeyCode::LShift) {
            self.translate(CameraDirection::Down, app_state.delta_time as f32);
        }

        if app_state.input.is_key_pressed(VirtualKeyCode::Space) {
            self.translate(CameraDirection::Up, app_state.delta_time as f32);
        }

        let offset = app_state.input.mouse.offset_from_center;
        self.process_mouse_movement(offset.x, offset.y);
    }
}

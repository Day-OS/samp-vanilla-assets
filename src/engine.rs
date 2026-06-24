pub struct WorldPosition {
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
    pub world_id: i32,
    pub interior_id: i32,
}

impl WorldPosition {
    pub fn position(&self) -> (f32, f32, f32) {
        (self.position_x, self.position_y, self.position_z)
    }
    pub fn rotation(&self) -> (f32, f32, f32) {
        (self.rotation_x, self.rotation_y, self.rotation_z)
    }
}

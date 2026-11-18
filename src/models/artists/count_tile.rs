/// Une tuile du bilan : séparés, gardés entiers ou à trancher.
pub struct CountTile {
    /// « 1 234 ».
    pub value: String,
    pub label: String,
    /// 0 séparés (vert), 1 entiers (neutre), 2 à trancher (ambre).
    pub tone: u8,
    pub icon: &'static str,
}

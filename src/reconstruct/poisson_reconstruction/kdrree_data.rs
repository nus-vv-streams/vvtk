#[derive(Clone)]
pub struct KdTreeData {
    pub index: usize,
    pub color: [u8; 4], // Assuming RGB values are 8-bit
}

impl PartialEq for KdTreeData {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

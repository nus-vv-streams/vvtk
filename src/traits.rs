use crate::points::Points;

pub trait ColorRecovery {
    fn nearest_point_recovery(self, data: Points) -> Points;
}
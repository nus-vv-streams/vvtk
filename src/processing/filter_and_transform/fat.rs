use crate::errors::*;

use crate::points::Points;

use crate::filter::FilterProducer;
use crate::transform::TransformProducer;

/// Filter and transform points
pub fn fat(
    points: &Points,
    filter_producer: Option<&FilterProducer>,
    transform_producer: Option<&TransformProducer>,
    transform_producer_remain: Option<&TransformProducer>,
) -> Result<Points> {
    let mut res = Points::new();
    let filter = filter_producer.chain_err(|| "Filter method not found")?(points);
    let change = transform_producer.chain_err(|| "Transform method not found")?(points);
    let change_remain =
        transform_producer_remain.chain_err(|| "Transform method for remain not found")?(points);

    for point in &points.data {
        if filter(point) {
            res.add(change(point))
        } else {
            res.add(change_remain(point))
        }
    }
    Ok(res)
}

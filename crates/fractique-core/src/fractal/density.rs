use super::Orbit;
use crate::random::{PROBE_POINTS, Probe, probe_of};
use crate::rng::Rng;

const PROBE_DOMAIN: u64 = 0x7000;

// The camera that frames the orbit and the 48 × 48 grid of where its points fall: what a randomizer judges by.
pub(crate) fn probe(orbit: &dyn Orbit) -> Option<Probe> {
    let mut rng = Rng::derive(0, PROBE_DOMAIN, 0);
    let mut points = Vec::with_capacity(PROBE_POINTS as usize);
    orbit
        .walk(&mut rng)
        .run(&mut rng, PROBE_POINTS, &mut |x, y, _| points.push((x, y)));
    points.retain(|(x, y)| x.is_finite() && y.is_finite());
    probe_of(points, false)
}

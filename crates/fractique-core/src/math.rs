//! Transcendental functions from the pure-Rust `libm` crate: the same bits on every platform.

pub const PI: f64 = core::f64::consts::PI;
pub const TAU: f64 = core::f64::consts::TAU;

#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}
#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}
#[inline]
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}
#[inline]
pub fn asin(x: f64) -> f64 {
    libm::asin(x)
}
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}
#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}
#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}
#[inline]
pub fn ln_1p(x: f64) -> f64 {
    libm::log1p(x)
}
#[inline]
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}
#[inline]
pub fn sinh(x: f64) -> f64 {
    libm::sinh(x)
}
#[inline]
pub fn cosh(x: f64) -> f64 {
    libm::cosh(x)
}
#[inline]
pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}
#[inline]
pub fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}

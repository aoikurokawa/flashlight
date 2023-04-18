// The code below is a stub. Just enough to satisfy the compiler.
// In order to pass the tests you can add-to or change any of this code.

#[derive(Debug)]
pub struct Duration {
    seconds: u64,
    days: f64,
    years: f64,
}

impl From<u64> for Duration {
    fn from(s: u64) -> Self {
        Self {
            seconds: s,
            days: s as f64 / 60.0 * 60.0 * 24.0,
            years: s as f64 / (60.0 * 60.0 * 24.0 * 365.25),
        }
    }
}

pub trait Planet {
    fn years_during(d: &Duration) -> f64 {
        let years = format!("{:.2}", d.years);
        years.parse::<f64>().unwrap()
    }
}

pub struct Mercury;
impl Planet for Mercury {}

pub struct Venus;
impl Planet for Venus{
    fn years_during(d: &Duration) -> f64 {
        let years = format!("{:.2}", d.years);
        years.parse::<f64>().unwrap() * 0.61519726
    }
}

pub struct Earth;
impl Planet for Earth{}

pub struct Mars;
impl Planet for Mars{}

pub struct Jupiter;
impl Planet for Jupiter {}

pub struct Saturn;
impl Planet for Saturn {}

pub struct Uranus;
impl Planet for Uranus {}

pub struct Neptune;
impl Planet for Neptune {}

#[cfg(test)]
mod tests {
    use crate::space_age::*;
    fn assert_in_delta(expected: f64, actual: f64) {
        let diff: f64 = (expected - actual).abs();
        let delta: f64 = 0.01;
        if diff > delta {
            panic!("Your result of {actual} should be within {delta} of the expected result {expected}")
        }
    }
    #[test]
    fn earth_age() {
        let duration = Duration::from(1_000_000_000);
        assert_in_delta(31.69, Earth::years_during(&duration));
    }
    #[test]
    fn mercury_age() {
        let duration = Duration::from(2_134_835_688);
        assert_in_delta(280.88, Mercury::years_during(&duration));
    }
    #[test]
    fn venus_age() {
        let duration = Duration::from(189_839_836);
        assert_in_delta(9.78, Venus::years_during(&duration));
    }
    #[test]
    fn mars_age() {
        let duration = Duration::from(2_129_871_239);
        assert_in_delta(35.88, Mars::years_during(&duration));
    }
    #[test]
    fn jupiter_age() {
        let duration = Duration::from(901_876_382);
        assert_in_delta(2.41, Jupiter::years_during(&duration));
    }
    #[test]
    fn saturn_age() {
        let duration = Duration::from(2_000_000_000);
        assert_in_delta(2.15, Saturn::years_during(&duration));
    }
    #[test]
    fn uranus_age() {
        let duration = Duration::from(1_210_123_456);
        assert_in_delta(0.46, Uranus::years_during(&duration));
    }
    #[test]
    fn neptune_age() {
        let duration = Duration::from(1_821_023_456);
        assert_in_delta(0.35, Neptune::years_during(&duration));
    }
}

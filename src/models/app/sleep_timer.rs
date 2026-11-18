/// Minuteur de veille : une durée ou la fin du morceau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepTimer {
    Minutes(u32),
    TrackEnd,
}

use crate::models::updates::*;

pub struct Slide {
    pub id: &'static str,
    pub icon: &'static str,
    pub colors: [&'static str; 2],
    pub satellites: &'static [&'static str],
    pub logos: &'static [(&'static str, &'static str)],
    pub action: Option<SlideGo>,
}

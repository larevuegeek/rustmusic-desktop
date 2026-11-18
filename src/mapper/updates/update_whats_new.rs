use crate::models::updates::*;
use karbon_ui::ui::animation::Effect;
use karbon_ui::ui::prelude::{Background, Color, GradientStop, LinearGradient, RadialExtent, RadialGradient};
use karbon_ui::ui::rendering::component_shadow::{BoxShadow, Shadows};
use karbon_ui::ui::styling::{color_mix, MixSpace};
use crate::helper::color::hex;
use karbon_ui::storage;

pub(crate) const fn slide(
    id: &'static str,
    icon: &'static str,
    colors: [&'static str; 2],
    satellites: &'static [&'static str],
    logos: &'static [(&'static str, &'static str)],
    action: Option<SlideGo>,
) -> Slide {
    Slide { id, icon, colors, satellites, logos, action }
}

/// Version présentée.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const VIEWED_KEY: &str = "rm-whatsnew-viewed";

/// `rangee` : un élément sur deux, répété pour remplir la largeur (le gabarit la double
/// pour boucler).
pub fn logo_rows(slide: &Slide) -> Vec<Vec<(&'static str, &'static str)>> {
    if slide.logos.is_empty() {
        return Vec::new();
    }
    (0..2)
        .map(|start_point| {
            let half: Vec<_> = slide.logos.iter().enumerate().filter(|(i, _)| i % 2 == start_point).map(|(_, l)| *l).collect();
            let base = if half.is_empty() { slide.logos.to_vec() } else { half };
            let mut out = base.clone();
            while out.len() < 9 {
                out.extend(base.iter().copied());
            }
            out
        })
        .collect()
}

/// `style="transform: translateX(-{index * 100}%)"` de la piste des diapos.
pub fn track(index: usize) -> Effect {
    Effect::new().translate_percent(-100. * index as f32, 0.)
}

/// `color-mix(in oklab, a p%, b)`.
pub(crate) fn mix(a: Color, b: Color, p: f32) -> Color {
    color_mix(MixSpace::Oklab, a, Some(p * 100.), b, None)
}

pub(crate) const WHITE: Color = Color::rgba(255, 255, 255, 255);

pub(crate) const BLACK: Color = Color::rgba(0, 0, 0, 255);

pub(crate) fn clear(c: Color) -> Color {
    Color { alpha: 0, ..c }
}

/// Fond de l'illustration : deux taches radiales sur un dégradé à 135°.
pub fn art_background(slide: &Slide) -> Vec<Background> {
    let (c0, c1) = (hex(slide.colors[0]), hex(slide.colors[1]));
    let a = mix(c0, WHITE, 0.85);
    let b = mix(c1, BLACK, 0.8);
    vec![
        RadialGradient::new([0., 0.], [0.9, 1.2], [GradientStop { position: 0., color: a }, GradientStop { position: 0.6, color: clear(a) }]).into(),
        RadialGradient::new([1., 1.], [0.8, 1.1], [GradientStop { position: 0., color: b }, GradientStop { position: 0.65, color: clear(b) }]).into(),
        LinearGradient::new(45., [GradientStop { position: 0., color: c0 }, GradientStop { position: 1., color: c1 }]).into(),
    ]
}

/// `.task-a` (0) et `.task-b` (1) : `radial-gradient(closest-side, mélange, transparent)`.
pub fn art_blob(slide: &Slide, which: usize) -> Vec<Background> {
    let c = if which == 0 { mix(hex(slide.colors[0]), WHITE, 0.55) } else { mix(hex(slide.colors[1]), WHITE, 0.7) };
    vec![RadialGradient::new([0.5, 0.5], [0.5, 0.5], [GradientStop { position: 0., color: c }, GradientStop { position: 1., color: clear(c) }])
        .extent(RadialExtent::ClosestSide, false)
        .into()]
}

/// `.projecteur` : `radial-gradient(180px 150px at 50% 50%, mélange 38%, transparent 100%)`.
pub fn art_spot(slide: &Slide) -> Vec<Background> {
    let c = mix(hex(slide.colors[0]), hex(slide.colors[1]), 0.92);
    vec![RadialGradient::new([0.5, 0.5], [180., 150.], [GradientStop { position: 0.38, color: c }, GradientStop { position: 1., color: clear(c) }])
        .extent(RadialExtent::Absolute, false)
        .into()]
}

/// Ombres de `.verre` (pastilles, satellites) et de `.centre`, dont la dernière est
/// teintée par la couleur de la diapo.
pub fn art_shadows(slide: &Slide, centre: bool) -> Shadows {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<(&'static str, bool), &'static [BoxShadow]>>> = OnceLock::new();
    let values = *CACHE.get_or_init(Default::default).lock().unwrap().entry((slide.id, centre)).or_insert_with(|| {
        // `color-mix(in oklab, var(--c1) 45%, rgb(0 0 0 / 0.5))` (.verre) et
        // `color-mix(in oklab, var(--c1) 55%, rgb(0 0 0 / 0.6))` (.centre).
        let (p, a) = if centre { (55., 0.6) } else { (45., 0.5) };
        let tint = color_mix(MixSpace::Oklab, hex(slide.colors[1]), Some(p), Color::rgba(0, 0, 0, (a * 255f32).round() as u8), None);
        let white = |a: f32| Color::rgba(255, 255, 255, (a * 255.).round() as u8);
        let shadow = |x: f32, y: f32, blur: f32, spread: f32, color: Color, inset: bool| BoxShadow { x, y, blur, spread, color, inset };
        let v = if centre {
            vec![
                shadow(0., 1.5, 0., 0., white(0.7), true),
                shadow(0., -10., 24., 0., white(0.1), true),
                shadow(0., 0., 0., 1., white(0.35), false),
                shadow(0., 26., 60., 0., tint, false),
            ]
        } else {
            vec![
                shadow(0., 1., 0., 0., white(0.55), true),
                shadow(0., 0., 0., 1., white(0.28), false),
                shadow(0., 10., 24., 0., tint, false),
            ]
        };
        Box::leak(v.into_boxed_slice())
    });
    Shadows::new(values)
}

/// Bouton « C'est parti » / « Suivant » : `linear-gradient(135deg, c0, c1)`.
pub fn next_background(slide: &Slide) -> Vec<Background> {
    vec![LinearGradient::new(45., [GradientStop { position: 0., color: hex(slide.colors[0]) }, GradientStop { position: 1., color: hex(slide.colors[1]) }]).into()]
}

impl crate::app::music_app::MusicApp {
    /// La fenêtre des nouveautés, s'il y a des diapos à montrer.
    pub fn whats_new_view(&self) -> Option<WhatsNewView> {
        use karbon_ui::intl::{t, t_with};
        let slides = crate::mapper::updates::update_releases::slides();
        let index = self.views.whats_new.index.min(slides.len().saturating_sub(1));
        let &(current, recalled) = slides.get(index)?;
        let count = slides.len();
        let kicker = match recalled {
            Some(version) => t_with("whats_new.kicker_since", &[("version", version)]),
            None => t_with("whats_new.kicker", &[("version", VERSION)]),
        };
        Some(WhatsNewView {
            index,
            kicker,
            slides: slides
                .iter()
                .enumerate()
                .map(|(i, &(slide, _))| WhatsNewSlideView {
                    index: i,
                    slide,
                    active: i == index,
                    position: format!("{} / {count}", i + 1),
                    title: t(&format!("whats_new.slides.{}.title", slide.id)),
                    text: t(&format!("whats_new.slides.{}.text", slide.id)),
                })
                .collect(),
            current,
            previous: index.checked_sub(1),
            last: index + 1 == count,
            at_startup: self.data.flag(crate::controller::updates::update_whats_new::AT_STARTUP),
        })
    }
}

impl WhatsNewState {
    pub fn load() -> Self {
        WhatsNewState { index: 0, viewed: storage::local().get(VIEWED_KEY).as_deref() == Some(VERSION) }
    }
}

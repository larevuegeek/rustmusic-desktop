//! Anneau de progression du chargeur d'import : piste et arc peints en polygones (le
//! cercle en pointillés de la 0.4 demandait `stroke-dasharray`).
use crate::app::resources::assets::Assets;
use karbon_ui::ui::element::{PaintContext, SizeF, TextMeasurer};
use karbon_ui::ui::prelude::*;
use karbon_ui::ui::scene::{DrawCommand, PointF, Scene};

/// Anneau de `size` px, rempli à `percent` (0 à 100) depuis le haut, dans le sens horaire.
pub struct ProgressRing {
    pub percent: f32,
    pub size: f32,
}

impl Default for ProgressRing {
    fn default() -> Self {
        Self { percent: 0., size: 128. }
    }
}

impl Component<Assets> for ProgressRing {
    fn render(self, cx: &TemplateContext<'_, Assets>, _: Slots) -> Element {
        Element::custom(cx.id.0, Ring { percent: self.percent.clamp(0., 100.), size: self.size })
    }
}

struct Ring {
    percent: f32,
    size: f32,
}

/// Points d'un secteur d'anneau de `from` à `to` (radians), contour extérieur puis
/// intérieur : un polygone simple, plein.
fn sector(center: PointF, outer: f32, inner: f32, from: f32, to: f32) -> Vec<PointF> {
    let steps = (((to - from).abs() / std::f32::consts::TAU) * 96.).ceil().max(2.) as usize;
    let at = |r: f32, a: f32| PointF { x: center.x + r * a.cos(), y: center.y + r * a.sin() };
    let mut points = Vec::with_capacity(2 * steps + 2);
    for i in 0..=steps {
        points.push(at(outer, from + (to - from) * i as f32 / steps as f32));
    }
    for i in (0..=steps).rev() {
        points.push(at(inner, from + (to - from) * i as f32 / steps as f32));
    }
    points
}

/// Disque plein (extrémité arrondie de l'arc).
fn disc(center: PointF, radius: f32) -> Vec<PointF> {
    (0..24)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 24.;
            PointF { x: center.x + radius * a.cos(), y: center.y + radius * a.sin() }
        })
        .collect()
}

impl Widget for Ring {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> karbon_ui::Result<SizeF> {
        Ok(SizeF { width: self.size, height: self.size })
    }

    fn paint(&self, ctx: &PaintContext, scene: &mut Scene) {
        let r = ctx.rect;
        let side = (r.right - r.left).min(r.bottom - r.top);
        let center = PointF { x: (r.left + r.right) / 2., y: (r.top + r.bottom) / 2. };
        // Mêmes proportions que le SVG de la 0.4 : rayon 42 et traits de 3 / 3,5 sur 100.
        let radius = side * 0.42;
        let track = side * 0.03;
        let arc = side * 0.035;
        let t = ctx.theme.text;
        let track_color = Color::rgba(t.red, t.green, t.blue, 16);
        let start = -std::f32::consts::FRAC_PI_2;
        // Piste en deux moitiés : chacune est un polygone simple.
        for half in 0..2 {
            let from = start + half as f32 * std::f32::consts::PI;
            scene.commands.push(DrawCommand::Path {
                points: sector(center, radius + track / 2., radius - track / 2., from, from + std::f32::consts::PI),
                closed: true,
                color: track_color,
            });
        }
        if self.percent <= 0. {
            return;
        }
        let end = start + std::f32::consts::TAU * self.percent / 100.;
        let accent = ctx.theme.accent;
        // Arc en morceaux d'au plus un demi-tour, puis bouts arrondis.
        let mut from = start;
        while from < end - 1e-4 {
            let to = (from + std::f32::consts::PI).min(end);
            scene.commands.push(DrawCommand::Path {
                points: sector(center, radius + arc / 2., radius - arc / 2., from, to),
                closed: true,
                color: accent,
            });
            from = to;
        }
        for a in [start, end] {
            scene.commands.push(DrawCommand::Path {
                points: disc(PointF { x: center.x + radius * a.cos(), y: center.y + radius * a.sin() }, arc / 2.),
                closed: true,
                color: accent,
            });
        }
    }
}

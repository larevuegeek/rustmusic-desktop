//! Couleur d'accent (Apparence) : palette de onze nuances calculée en OKLCH depuis la
//! couleur choisie, le niveau 500 étant la couleur elle-même (portage de `accent.ts` de la
//! 0.4). Les classes `emerald-*` et `accent-*` lisent ces nuances (`--acc-*`, `app.css`),
//! les verts du thème et de la barre latérale suivent (`:root[data-accent]` de la 0.4).
//! Vide ou vert d'origine : rien n'est posé, la palette d'origine reste.

use karbon_ui::ui::prelude::*;

/// Vert de la marque : choisi, il laisse la palette d'origine.
const DEFAULT_ACCENT: u32 = 0x22c55e;
const SHADES: [&str; 11] = ["50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"];
/// Écarts de clarté et de chroma autour du 500, relevés sur la palette verte de Tailwind.
const LIGHTNESS_DELTA: [f32; 11] = [0.259, 0.239, 0.202, 0.148, 0.069, 0., -0.096, -0.196, -0.275, -0.33, -0.457];
const CHROMA_RATIO: [f32; 11] = [0.08, 0.2, 0.38, 0.68, 0.95, 1., 0.89, 0.7, 0.54, 0.43, 0.3];
/// Fonds sur lesquels les verts pâles du thème sont mélangés (clair, sombre, creux sombre).
const LIGHT_GROUND: u32 = 0xf8f6f1;
const DARK_GROUND: u32 = 0x0b0d0c;
const DARK_HOLLOW: u32 = 0x0f1211;

/// Accent choisi (réglage `accent_color`, `#rrggbb`), `None` pour le vert d'origine ou une
/// valeur illisible.
pub(crate) fn custom(setting: Option<&str>) -> Option<u32> {
    let hex = setting?.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok().filter(|rgb| *rgb != DEFAULT_ACCENT)
}

/// Variables à poser pour l'accent `rgb` : la palette `--acc-*`, `--acc-on`, puis les verts
/// du thème (`--rg-*`) et de la barre latérale (`--sb-*`) du mode clair ou sombre.
pub(crate) fn variables(rgb: u32, dark: bool) -> Vec<(String, Color)> {
    let shades = palette(rgb);
    let shade = |name: &str| shades.iter().find(|(n, _)| *n == name).map_or(rgb, |(_, c)| *c);
    let mix = |percent: f32, ground: u32| mix_oklab(shade("500"), ground, percent / 100.);
    let mut out: Vec<(String, u32)> = shades.iter().map(|(n, c)| (format!("--acc-{n}"), *c)).collect();
    // Texte posé sur l'accent : sombre sur une couleur claire, blanc sinon (le 600 sert en
    // thème clair).
    let on = if to_oklch(shade("600")).l >= 0.6 {
        from_oklch(Oklch { l: 0.18, c: 0.04, h: to_oklch(rgb).h })
    } else {
        0xffffff
    };
    out.push(("--acc-on".into(), on));
    out.push(("--rg-on-g".into(), on));
    let themed: [(&str, u32); 9] = if dark {
        [
            ("--rg-g", shade("500")),
            ("--rg-gtx", shade("400")),
            ("--rg-gbg", mix(18., DARK_GROUND)),
            ("--rg-gbd", mix(34., DARK_GROUND)),
            ("--rg-creux-on", mix(8., DARK_HOLLOW)),
            ("--sb-g", shade("400")),
            ("--sb-point", shade("500")),
            ("--sb-gbg", mix(18., DARK_GROUND)),
            ("--sb-gbd", mix(30., DARK_GROUND)),
        ]
    } else {
        [
            ("--rg-g", shade("600")),
            ("--rg-gtx", shade("800")),
            ("--rg-gbg", mix(16., LIGHT_GROUND)),
            ("--rg-gbd", mix(32., LIGHT_GROUND)),
            ("--rg-creux-on", mix(7., LIGHT_GROUND)),
            ("--sb-g", shade("800")),
            ("--sb-point", shade("600")),
            ("--sb-gbg", mix(16., LIGHT_GROUND)),
            ("--sb-gbd", mix(16., LIGHT_GROUND)),
        ]
    };
    out.extend(themed.iter().map(|(n, c)| (n.to_string(), *c)));
    out.into_iter().map(|(n, c)| (n, Color::hex(c))).collect()
}

/// Les onze nuances (`50` … `950`) de l'accent `rgb`.
fn palette(rgb: u32) -> Vec<(&'static str, u32)> {
    let base = to_oklch(rgb);
    SHADES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let d = LIGHTNESS_DELTA[i];
            // Clairs jusqu'à 0,985, sombres jusqu'à 0,2 : la palette reste ordonnée quelle
            // que soit la couleur.
            let l = if d >= 0. { base.l + d * ((0.985 - base.l) / 0.259) } else { base.l + d * ((base.l - 0.2) / 0.457) };
            let color = if *name == "500" { rgb } else { from_oklch(Oklch { l, c: base.c * CHROMA_RATIO[i], h: base.h }) };
            (*name, color)
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct Oklch {
    l: f32,
    c: f32,
    h: f32,
}

fn to_linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn from_linear(v: f32) -> f32 {
    if v <= 0.003_130_8 { 12.92 * v } else { 1.055 * v.powf(1. / 2.4) - 0.055 }
}

fn to_oklab(rgb: u32) -> [f32; 3] {
    let channel = |shift: u32| to_linear(((rgb >> shift) & 255) as f32 / 255.);
    let (r, g, b) = (channel(16), channel(8), channel(0));
    let l = (0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn to_oklch(rgb: u32) -> Oklch {
    let [l, a, b] = to_oklab(rgb);
    Oklch { l, c: a.hypot(b), h: (b.atan2(a).to_degrees() + 360.) % 360. }
}

/// RGB linéaire, hors de [0, 1] si la couleur sort de sRGB.
fn linear_rgb([l, a, b]: [f32; 3]) -> [f32; 3] {
    let l_ = (l + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m_ = (l - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s_ = (l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    [
        4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_,
        -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_,
    ]
}

fn pack(linear: [f32; 3]) -> u32 {
    linear
        .iter()
        .map(|v| (from_linear(v.clamp(0., 1.)) * 255.).round().clamp(0., 255.) as u32)
        .fold(0, |acc, v| (acc << 8) | v)
}

/// En sRGB : la chroma baisse jusqu'à ce que la nuance soit affichable.
fn from_oklch(color: Oklch) -> u32 {
    let lab = |c: f32| {
        let h = color.h.to_radians();
        [color.l, c * h.cos(), c * h.sin()]
    };
    let mut c = color.c;
    let mut rgb = linear_rgb(lab(c));
    for _ in 0..24 {
        if rgb.iter().all(|v| (-0.0001..=1.0001).contains(v)) {
            break;
        }
        c *= 0.92;
        rgb = linear_rgb(lab(c));
    }
    pack(rgb)
}

/// `color-mix(in oklab, a p, b)`, `p` entre 0 et 1.
fn mix_oklab(a: u32, b: u32, p: f32) -> u32 {
    let (a, b) = (to_oklab(a), to_oklab(b));
    pack(linear_rgb([0, 1, 2].map(|i| a[i] * p + b[i] * (1. - p))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brand_green_and_invalid_values_keep_the_original_palette() {
        assert_eq!(custom(Some("#22c55e")), None);
        assert_eq!(custom(Some("")), None);
        assert_eq!(custom(Some("bleu")), None);
        assert_eq!(custom(None), None);
        assert_eq!(custom(Some("#3B82F6")), Some(0x3b82f6));
    }

    #[test]
    fn the_palette_keeps_the_choice_at_500_and_is_ordered_light_to_dark() {
        let shades = palette(0x3b82f6);
        assert_eq!(shades[5], ("500", 0x3b82f6));
        let lightness: Vec<f32> = shades.iter().map(|(_, c)| to_oklch(*c).l).collect();
        assert!(lightness.windows(2).all(|w| w[0] > w[1]), "{lightness:?}");
    }

    #[test]
    fn converting_back_and_forth_keeps_the_colour() {
        for rgb in [0x22c55e, 0x3b82f6, 0xef4444, 0xf59e0b] {
            assert_eq!(from_oklch(to_oklch(rgb)), rgb);
        }
    }
}

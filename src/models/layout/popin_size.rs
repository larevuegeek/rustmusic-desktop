/// `size` : largeur maximale (`max-w-md`, `max-w-2xl`, `max-w-3xl`, `max-w-[980px]`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PopinSize {
    #[default]
    Md,
    Lg,
    Xl,
    Xxl,
}

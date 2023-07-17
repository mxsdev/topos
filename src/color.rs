use palette::*;

pub type ColorRgba = LinSrgba;
pub type ColorRgb = LinSrgb;

pub type ColorSrgba<T = f32> = Srgba<T>;
pub type ColorSrgb<T = f32> = Srgb<T>;

// TODO: eventually remove this
pub trait FromCosmicTextColor {
    fn from_cosmic(col: cosmic_text::Color) -> Self;
}

pub trait IntoCosmicTextColor {
    fn into_cosmic(self) -> cosmic_text::Color;
}

impl IntoCosmicTextColor for ColorRgba {
    fn into_cosmic(self) -> cosmic_text::Color {
        cosmic_text::Color(self.into_format().into())
    }
}

impl FromCosmicTextColor for ColorRgba {
    fn from_cosmic(col: cosmic_text::Color) -> Self {
        ColorSrgba::from(col.0).into()
    }
}

impl IntoCosmicTextColor for ColorSrgba<f32> {
    fn into_cosmic(self) -> cosmic_text::Color {
        cosmic_text::Color(self.into_format().into())
    }
}

impl FromCosmicTextColor for ColorSrgba<f32> {
    fn from_cosmic(col: cosmic_text::Color) -> Self {
        ColorSrgba::from(col.0).into_format()
    }
}

impl IntoCosmicTextColor for ColorSrgba<u8> {
    fn into_cosmic(self) -> cosmic_text::Color {
        cosmic_text::Color(self.into_format().into())
    }
}

impl FromCosmicTextColor for ColorSrgba<u8> {
    fn from_cosmic(col: cosmic_text::Color) -> Self {
        ColorSrgba::from(col.0).into_format()
    }
}

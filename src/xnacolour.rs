use {
    palette::{
        convert::{FromColorUnclamped, IntoColorUnclamped},
        rgb::Rgb,
        Srgb, WithAlpha,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, PartialEq, Debug, FromColorUnclamped, WithAlpha, Copy, Clone)]
#[palette(skip_derives(Rgb), rgb_standard = "palette::encoding::Srgb")]
pub struct XNAColour {
    red: u8,
    green: u8,
    blue: u8,
    #[palette(alpha)]
    alpha: f32,
}

impl XNAColour {

    pub fn imgcolor(self) -> [f32; 4] {
        let srgb: Srgb = self.into_color_unclamped();
        //let alpha = 1.0 - self.alpha;
        [srgb.red, srgb.blue, srgb.green, 1.0]
    }
}

impl<S> FromColorUnclamped<Rgb<S, f32>> for XNAColour
where
    Srgb<f32>: FromColorUnclamped<Rgb<S, f32>>,
{
    fn from_color_unclamped(color: Rgb<S, f32>) -> XNAColour {
        let srgb = Srgb::from_color_unclamped(color).into_format();

        XNAColour {
            red: srgb.red,
            green: srgb.green,
            blue: srgb.blue,
            alpha: 1.0,
        }
    }
}

impl<S> FromColorUnclamped<XNAColour> for Rgb<S, f32>
where
    Srgb<f32>: IntoColorUnclamped<Rgb<S, f32>>,
{
    fn from_color_unclamped(color: XNAColour) -> Rgb<S, f32> {
        Srgb::new(color.red, color.green, color.blue)
            .into_format()
            .into_color_unclamped()
    }
}

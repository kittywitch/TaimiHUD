use palette::rgb::Rgb;
use palette::convert::{FromColorUnclamped, IntoColorUnclamped};
use palette::{Srgba,Srgb,WithAlpha};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, PartialEq, Debug, FromColorUnclamped, WithAlpha, Clone)]
#[palette(
    skip_derives(Rgb),
    rgb_standard = "palette::encoding::Srgb"
)]
pub struct XNAColour {
    red: u8,
    green: u8,
    blue: u8,
    #[palette(alpha)]
    alpha: f32,
}

impl<S> FromColorUnclamped<Rgb<S, f32>> for XNAColour
where
    Srgb<f32>: FromColorUnclamped<Rgb<S, f32>>
{
    fn from_color_unclamped(color: Rgb<S, f32>) -> XNAColour{
        let srgb = Srgb::from_color_unclamped(color)
            .into_format();

        XNAColour {
            red: srgb.red,
            green: srgb.green,
            blue: srgb.blue,
            alpha: 1.0
        }
    }
}

impl<S> FromColorUnclamped<XNAColour> for Rgb<S, f32>
where
    Srgb<f32>: IntoColorUnclamped<Rgb<S, f32>>
{
    fn from_color_unclamped(color: XNAColour) -> Rgb<S, f32>{
        Srgb::new(color.red, color.green, color.blue)
            .into_format()
            .into_color_unclamped()
    }
}

/*
pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<XNAColour, D::Error> {
    let (r, g, b, a) = <(u8, u8, u8, f32)>::deserialize(de)?;

    Ok(XNAColour { red: r,
        green: g,
        blue: b,
        alpha: a
        })
}

pub fn serialize<S: Serializer>(colour: &XNAColour, ser: S) -> Result<S::Ok, S::Error> {
    let colour = (colour.red, colour.green, colour.blue, colour.alpha);

    colour.serialize(ser)
}*/

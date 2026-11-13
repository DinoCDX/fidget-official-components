use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: u8::MAX }
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self {
            a: (alpha.clamp(0.0, 1.0) * f32::from(u8::MAX)).round() as u8,
            ..self
        }
    }
}

#[derive(Debug, Error)]
#[error("color must be #RRGGBB or #RRGGBBAA, got `{0}`")]
pub struct ColorParseError(pub String);

impl FromStr for Rgba {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(s.to_owned());
        let hex = s.strip_prefix('#').ok_or_else(err)?;
        let channel = |i: usize| {
            hex.get(i..i + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(err)
        };
        match hex.len() {
            6 => Ok(Self {
                r: channel(0)?,
                g: channel(2)?,
                b: channel(4)?,
                a: u8::MAX,
            }),
            8 => Ok(Self {
                r: channel(0)?,
                g: channel(2)?,
                b: channel(4)?,
                a: channel(6)?,
            }),
            _ => Err(err()),
        }
    }
}

impl TryFrom<String> for Rgba {
    type Error = ColorParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Rgba> for String {
    fn from(color: Rgba) -> Self {
        color.to_string()
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == u8::MAX {
            write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            write!(f, "#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Rgba;

    #[test]
    fn parses_rgb_and_rgba() {
        assert_eq!("#6750A4".parse::<Rgba>().unwrap(), Rgba::rgb(0x67, 0x50, 0xA4));
        assert_eq!("#00000080".parse::<Rgba>().unwrap().a, 0x80);
    }

    #[test]
    fn rejects_malformed_input() {
        assert!("6750A4".parse::<Rgba>().is_err());
        assert!("#12345".parse::<Rgba>().is_err());
        assert!("#GG0000".parse::<Rgba>().is_err());
    }
}

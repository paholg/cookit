use {
    serde::{Deserialize, Serialize},
    std::str::FromStr,
    strum::{Display, EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator},
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Mass {
    G,
    Kg,
    Mg,
    Oz,
    Lb,
}

impl Mass {
    /// Multiplier to canonical grams.
    pub fn grams(self) -> f64 {
        match self {
            Mass::G => 1.0,
            Mass::Kg => 1000.0,
            Mass::Mg => 0.001,
            Mass::Oz => 28.35,
            Mass::Lb => 453.59,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Volume {
    Ml,
    L,
    Tsp,
    Tbsp,
    #[strum(serialize = "fl oz")]
    FlOz,
    Cup,
    Pt,
    Qt,
    Gal,
}

impl Volume {
    /// Multiplier to canonical milliliters.
    pub fn ml(self) -> f64 {
        match self {
            Volume::Ml => 1.0,
            Volume::L => 1000.0,
            Volume::Tsp => 4.93,
            Volume::Tbsp => 14.79,
            Volume::FlOz => 29.57,
            Volume::Cup => 236.59,
            Volume::Pt => 473.18,
            Volume::Qt => 946.35,
            Volume::Gal => 3_785.41,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(
    name(UnitKind),
    derive(Hash, Serialize, Deserialize, Display, EnumString),
    strum(serialize_all = "lowercase", ascii_case_insensitive),
    serde(rename_all = "lowercase")
)]
pub enum Unit {
    Mass(Mass),
    Volume(Volume),
    Count(String),
    Custom(String),
}

impl Unit {
    /// Build a `Unit` from a kind selector and the user-typed unit text.
    /// For Mass/Volume, the text must name a known unit (case-insensitive).
    pub fn new(kind: UnitKind, text: &str) -> Result<Self, String> {
        let t = text.trim();
        match kind {
            UnitKind::Mass => Mass::from_str(t).map(Unit::Mass).map_err(|_| {
                format!(
                    "unknown mass unit `{t}`; known: {}",
                    Mass::iter()
                        .map(|u| u.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }),
            UnitKind::Volume => Volume::from_str(t).map(Unit::Volume).map_err(|_| {
                format!(
                    "unknown volume unit `{t}`; known: {}",
                    Volume::iter()
                        .map(|u| u.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }),
            UnitKind::Count => Ok(Unit::Count(t.to_string())),
            UnitKind::Custom => Ok(Unit::Custom(t.to_string())),
        }
    }

    pub fn kind(&self) -> UnitKind {
        self.into()
    }

    pub fn label(&self) -> String {
        match self {
            Unit::Mass(m) => m.to_string(),
            Unit::Volume(v) => v.to_string(),
            Unit::Count(s) | Unit::Custom(s) => s.clone(),
        }
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

/// Interpret the free-form unit text. A known mass/volume unit keeps its kind;
/// anything else is treated as a count label. Empty means no unit.
pub fn parse_unit(text: &str) -> Option<Unit> {
    let t = text.trim();

    if t.is_empty() {
        None
    } else if let Ok(m) = Mass::from_str(t) {
        Some(Unit::Mass(m))
    } else if let Ok(v) = Volume::from_str(t) {
        Some(Unit::Volume(v))
    } else {
        Some(Unit::Count(t.to_string()))
    }
}

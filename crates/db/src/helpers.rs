use {
    serde::{Deserialize, Serialize},
    snafu::{Snafu, ensure},
};

/// A user-correctable input error: the message is meant to be shown next to
/// the offending field.
#[derive(Debug, Snafu)]
#[snafu(display("Validation error: {msg}"))]
pub struct ValidationError {
    pub msg: String,
}

/// URL-safe kebab-case slug of `name`. Lowercases ASCII letters/digits,
/// replaces runs of everything else with a single `-`, trims leading/trailing
/// `-`. Falls back to `"item"` if the result is empty.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;

    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.extend(c.to_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        "item".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_basics() {
        assert_eq!(slugify("Black Beans"), "black-beans");
        assert_eq!(slugify("  Chicken & Waffles  "), "chicken-waffles");
        assert_eq!(slugify("Mom's Chili (Spicy!)"), "mom-s-chili-spicy");
        assert_eq!(slugify("---"), "item");
        assert_eq!(slugify(""), "item");
        assert_eq!(slugify("café"), "caf");
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Text))]
pub struct Name(pub String);

impl Name {
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ValidationError> {
        let s = s.as_ref().trim().to_string();
        ensure!(
            !s.is_empty(),
            ValidationSnafu {
                msg: "name is required"
            }
        );
        Ok(Self(s))
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "server")]
mod name_diesel {
    use {
        super::Name,
        diesel::{
            backend::Backend,
            deserialize::{self, FromSql},
            pg::Pg,
            serialize::{self, Output, ToSql},
            sql_types::Text,
        },
    };

    impl ToSql<Text, Pg> for Name {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
            ToSql::<Text, Pg>::to_sql(self.0.as_str(), &mut out.reborrow())
        }
    }

    impl FromSql<Text, Pg> for Name {
        fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            Ok(Self(<String as FromSql<Text, Pg>>::from_sql(bytes)?))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Float8))]
pub struct PositiveFloat(pub f64);

impl PositiveFloat {
    pub fn parse(v: f64) -> anyhow::Result<Self> {
        if !v.is_finite() || v <= 0.0 {
            anyhow::bail!("must be a positive number, got {v}");
        }
        Ok(Self(v))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

#[cfg(feature = "server")]
mod positive_float_diesel {
    use {
        super::PositiveFloat,
        diesel::{
            backend::Backend,
            deserialize::{self, FromSql},
            pg::Pg,
            serialize::{self, Output, ToSql},
            sql_types::Float8,
        },
    };

    impl ToSql<Float8, Pg> for PositiveFloat {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
            ToSql::<Float8, Pg>::to_sql(&self.0, &mut out.reborrow())
        }
    }

    impl FromSql<Float8, Pg> for PositiveFloat {
        fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            Ok(Self(<f64 as FromSql<Float8, Pg>>::from_sql(bytes)?))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Text))]
pub struct Slug(String);

impl Slug {
    pub fn parse(s: impl AsRef<str>) -> Self {
        Self(slugify(s.as_ref()))
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "server")]
mod slug_diesel {
    use {
        super::Slug,
        diesel::{
            backend::Backend,
            deserialize::{self, FromSql},
            pg::Pg,
            serialize::{self, Output, ToSql},
            sql_types::Text,
        },
    };

    impl ToSql<Text, Pg> for Slug {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
            ToSql::<Text, Pg>::to_sql(self.0.as_str(), &mut out.reborrow())
        }
    }

    impl FromSql<Text, Pg> for Slug {
        fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            Ok(Self(<String as FromSql<Text, Pg>>::from_sql(bytes)?))
        }
    }
}

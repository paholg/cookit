use {crate::impl_diesel, email_address::EmailAddress, nutype::nutype, std::str::FromStr};

const SLUG_MIN_LEN: usize = 4;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_min = 4, regex = "^[a-z0-9_-]+$"),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        FromStr,
        AsRef,
        Deref,
        TryFrom,
        Into,
        Hash,
        Borrow,
        Display,
        Serialize,
        Deserialize
    )
)]
pub struct Slug(String);
impl_diesel!(Slug, String, Text);

#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        FromStr,
        AsRef,
        Deref,
        TryFrom,
        Into,
        Hash,
        Borrow,
        Display,
        Serialize,
        Deserialize
    )
)]
pub struct Name(String);
impl_diesel!(Name, String, Text);

impl Name {
    /// URL-safe kebab-case slug of `name`.
    pub fn slugify(&self) -> Result<Slug, SlugError> {
        let mut out = String::with_capacity(self.len());
        let mut last_dash = true;

        for c in self.chars() {
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
            out.push_str("item");
        }

        // A `Slug` must be at least `SLUG_MIN_LEN` chars; pad short slugs (e.g.
        // "pho") with trailing zeros so machine-generated slugs always validate.
        while out.chars().count() < SLUG_MIN_LEN {
            out.push('0');
        }

        Slug::try_new(out)
    }
}

// TODO: Deprecate.
pub fn slugify(s: &str) -> String {
    Name::try_from(s).unwrap().slugify().unwrap().into_inner()
}

#[nutype(
    validate(finite, greater = 0.0),
    derive(
        Debug,
        Copy,
        Clone,
        PartialEq,
        Eq,
        FromStr,
        AsRef,
        Deref,
        TryFrom,
        Into,
        Borrow,
        Display,
        Serialize,
        Deserialize
    )
)]
pub struct PositiveFloat(f64);
impl_diesel!(PositiveFloat, f64, Double);

#[nutype(derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromStr,
    AsRef,
    Deref,
    From,
    Into,
    Borrow,
    Display,
    Serialize,
    Deserialize
))]
pub struct Email(EmailAddress);

impl TryFrom<String> for Email {
    type Error = <EmailAddress as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let inner = EmailAddress::from_str(&value)?;
        Ok(Email::from(inner))
    }
}

#[cfg(feature = "server")]
impl diesel::expression::AsExpression<diesel::sql_types::Text> for Email {
    type Expression =
        <String as diesel::expression::AsExpression<diesel::sql_types::Text>>::Expression;

    fn as_expression(self) -> Self::Expression {
        diesel::expression::AsExpression::<diesel::sql_types::Text>::as_expression(
            self.into_inner().to_string(),
        )
    }
}

#[cfg(feature = "server")]
impl diesel::expression::AsExpression<diesel::sql_types::Text> for &Email {
    type Expression =
        <String as diesel::expression::AsExpression<diesel::sql_types::Text>>::Expression;

    fn as_expression(self) -> Self::Expression {
        diesel::expression::AsExpression::<diesel::sql_types::Text>::as_expression(
            self.as_str().to_string(),
        )
    }
}

#[cfg(feature = "server")]
impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::pg::Pg> for Email {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        let inner: &'b EmailAddress = self;
        <str as diesel::serialize::ToSql<diesel::sql_types::Text, diesel::pg::Pg>>::to_sql(
            inner.as_str(),
            out,
        )
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::pg::Pg> for Email {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let s = <String as diesel::deserialize::FromSql<
            diesel::sql_types::Text,
            diesel::pg::Pg,
        >>::from_sql(bytes)?;
        Ok(Email::new(s.parse::<EmailAddress>()?))
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::Queryable<diesel::sql_types::Text, diesel::pg::Pg> for Email {
    type Row = String;

    fn build(row: String) -> diesel::deserialize::Result<Self> {
        Ok(Email::new(row.parse::<EmailAddress>()?))
    }
}

#[nutype(derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    FromStr,
    AsRef,
    Deref,
    From,
    Into,
    Borrow,
    Display,
    Serialize,
    Deserialize
))]
pub struct Timestamp(jiff::Timestamp);

#[cfg(feature = "server")]
impl diesel::deserialize::FromSql<diesel::sql_types::Timestamptz, diesel::pg::Pg> for Timestamp {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let ts = <jiff_diesel::Timestamp as diesel::deserialize::FromSql<
            diesel::sql_types::Timestamptz,
            diesel::pg::Pg,
        >>::from_sql(bytes)?;
        Ok(Timestamp::new(jiff::Timestamp::from(ts)))
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::Queryable<diesel::sql_types::Timestamptz, diesel::pg::Pg> for Timestamp {
    type Row = jiff_diesel::Timestamp;

    fn build(row: jiff_diesel::Timestamp) -> diesel::deserialize::Result<Self> {
        Ok(Timestamp::new(jiff::Timestamp::from(row)))
    }
}

#[cfg(test)]
mod tests {
    use crate::Name;

    fn slugify(s: &str) -> String {
        Name::try_from(s).unwrap().slugify().unwrap().into_inner()
    }

    #[test]
    fn slugify_basics() {
        assert_eq!(slugify("Black Beans"), "black-beans");
        assert_eq!(slugify("  Chicken & Waffles  "), "chicken-waffles");
        assert_eq!(slugify("Mom's Chili (Spicy!)"), "mom-s-chili-spicy");
        assert_eq!(slugify("---"), "item");
        // TODO: We should handle utf-8 better.
        assert_eq!(slugify("café"), "caf0");
    }

    #[test]
    fn slugify_pads_short_slugs() {
        // Short results are right-padded with `0` to the slug minimum length so
        // they pass `Slug` validation.
        assert_eq!(slugify("Pho"), "pho0");
        assert_eq!(slugify("Hi"), "hi00");
        assert_eq!(slugify("OK!"), "ok00");

        for s in ["Pho", "Hi", "OK!", "café"] {
            assert!(crate::Slug::try_new(slugify(s)).is_ok());
        }
    }
}

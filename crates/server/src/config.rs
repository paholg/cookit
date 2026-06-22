use {
    crate::error::{MalformedHostSnafu, ValidationSnafu},
    db::{Slug, models::book::Book},
    figment::{Figment, providers::Env},
    serde::Deserialize,
    snafu::{IntoError, NoneError, OptionExt},
    std::sync::LazyLock,
    url::Url,
};

#[derive(Deserialize)]
pub struct Config {
    // pub session_secret: SecretString,
    pub database_url: Url,
    pub base_domain: String,
}

impl Config {
    fn new() -> Self {
        Figment::new().merge(Env::raw()).extract().unwrap()
    }

    pub fn book_slug(&self, host: &str) -> crate::Result<Option<Slug>> {
        let host = host.split(':').next().unwrap_or(host).trim().to_lowercase();
        let Some(prefix) = host
            .strip_suffix(&self.base_domain)
            .context(MalformedHostSnafu {
                host: host.to_string(),
                base: self.base_domain.clone(),
            })?
            .strip_suffix('.')
        else {
            return Ok(None);
        };

        let slug = Slug::try_from(prefix.to_string())
            .map_err(|e| ValidationSnafu { msg: e.to_string() }.into_error(NoneError))?;

        Ok(Some(slug))
    }

    pub fn host(&self, book: Option<&Book>) -> String {
        match book {
            Some(b) => format!("{}.{}", b.slug, self.base_domain),
            None => self.base_domain.to_string(),
        }
    }
}

static CONFIG: LazyLock<Config> = LazyLock::new(Config::new);

pub fn config() -> &'static Config {
    &CONFIG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_slug(host: &str, rhs: crate::Result<Option<&str>>) {
        let slug = Config {
            database_url: Url::parse("http://foo.foo").unwrap(),
            base_domain: "cookit.com".into(),
        }
        .book_slug(host);

        let slug_str = slug.as_ref().map(|opt| opt.as_ref().map(|s| s.as_str()));
        assert_eq!(slug_str, rhs.as_ref().copied());
    }

    #[test]
    fn book_slug_extracts_single_label_subdomain() {
        assert_slug("kitchen.cookit.com", Ok(Some("kitchen")));
    }

    #[test]
    fn book_slug_strips_port_and_case() {
        assert_slug("Kitchen.Cookit.COM:8080", Ok(Some("kitchen")));
    }

    #[test]
    fn book_slug_mismatch() {
        assert_slug("cookit.com", Ok(None));
        assert_slug("Cookit.COM:8080", Ok(None));
        assert_slug(
            "example.com",
            Err(crate::Error::MalformedHost {
                host: "example.com".into(),
                base: "cookit.com".into(),
            }),
        );
    }
}

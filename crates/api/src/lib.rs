pub mod config;
pub mod db;
pub mod middleware;
// FIXME
// pub mod ops;
pub mod grocery_section;
pub mod id;
pub mod session;
pub mod unit;

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

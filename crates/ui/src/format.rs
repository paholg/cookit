/// Format a recipe / shopping-list quantity as a short human-friendly string.
/// - Integers render without a decimal (`3` not `3.0`).
/// - Otherwise round to two decimal places and drop a trailing `.0`.
pub fn format_quantity(q: f64) -> String {
    if q.fract().abs() < f64::EPSILON {
        return format!("{}", q as i64);
    }

    let rounded = (q * 100.0).round() / 100.0;
    if rounded.fract().abs() < f64::EPSILON {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

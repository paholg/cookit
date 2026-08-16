//! A span per database query, via diesel's connection instrumentation.

use {
    diesel::connection::{Instrumentation, InstrumentationEvent, set_default_instrumentation},
    tracing::{Level, Span, field::Empty},
};

/// What deadpool's `Verified` recycling runs against every connection it hands
/// out. It says nothing about the app, and there are several per request.
const RECYCLE_PING: &str = "SELECT $1";

/// Makes every new connection report its queries as spans.
///
/// diesel calls this constructor once per connection, and a connection runs at
/// most one query at a time, so the in-flight span can live in the
/// instrumentation itself.
pub(super) fn install() {
    if let Err(error) = set_default_instrumentation(|| Some(Box::new(QuerySpans::default()))) {
        eprintln!("Failed to install database instrumentation; queries won't be traced: {error}");
    }
}

#[derive(Default)]
struct QuerySpans {
    in_flight: Option<Span>,
}

impl Instrumentation for QuerySpans {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        match event {
            InstrumentationEvent::StartQuery { query, .. } => {
                self.in_flight = None;

                // Rendering the query isn't cheap, and a span that is created
                // at all gets exported, so decide before doing either.
                if !tracing::enabled!(Level::INFO) {
                    return;
                }

                let rendered = query.to_string();
                let sql = statement(&rendered);
                if sql == RECYCLE_PING {
                    return;
                }

                let (operation, collection) = target_of(sql);

                self.in_flight = Some(tracing::info_span!(
                    "db_query",
                    otel.name = span_name(operation, collection),
                    otel.kind = "client",
                    otel.status_code = Empty,
                    otel.status_message = Empty,
                    db.system.name = "postgresql",
                    db.operation.name = operation,
                    // `None` records nothing, leaving the attribute off.
                    db.collection.name = collection,
                    db.query.text = sql,
                ));
            }
            InstrumentationEvent::FinishQuery { error, .. } => {
                // Dropping the span closes it, which is what bounds its
                // duration.
                let Some(span) = self.in_flight.take() else {
                    return;
                };

                if let Some(error) = error {
                    span.record("otel.status_code", "ERROR");
                    span.record("otel.status_message", error.to_string());
                }
            }
            _ => (),
        }
    }
}

/// Drops the `-- binds: [..]` comment diesel appends. Bind values can hold user
/// data, and `db.query.text` is meant to be the parameterized statement anyway.
fn statement(rendered: &str) -> &str {
    rendered
        .split_once(" -- binds:")
        .map_or(rendered, |(sql, _binds)| sql)
        .trim()
}

/// The `db.operation.name` and `db.collection.name` of a statement, as far as
/// they can be read off the front of it. The table is `None` for statements
/// with no obvious one, like `SET TIME ZONE 'UTC'`.
fn target_of(sql: &str) -> (&str, Option<&str>) {
    let mut words = sql.split_whitespace();
    let Some(operation) = words.next() else {
        return ("query", None);
    };

    let table = match operation.to_ascii_uppercase().as_str() {
        "SELECT" | "DELETE" => words
            .find(|word| word.eq_ignore_ascii_case("FROM"))
            .and_then(|_| words.next()),
        "INSERT" => words
            .find(|word| word.eq_ignore_ascii_case("INTO"))
            .and_then(|_| words.next()),
        "UPDATE" => words.next(),
        _ => None,
    };

    // diesel quotes identifiers, and the table can trail a `(` in inserts.
    (operation, table.map(|table| table.trim_matches(['"', '('])))
}

/// `SELECT recipes`, the span name the OpenTelemetry conventions ask for.
fn span_name(operation: &str, collection: Option<&str>) -> String {
    match collection {
        Some(collection) => format!("{operation} {collection}"),
        None => operation.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_drops_binds() {
        assert_eq!(
            statement(r#"SELECT "users"."id" FROM "users" WHERE "id" = $1 -- binds: [42]"#),
            r#"SELECT "users"."id" FROM "users" WHERE "id" = $1"#
        );
    }

    #[test]
    fn target_reads_operation_and_table() {
        assert_eq!(
            target_of(r#"SELECT "recipes"."id" FROM "recipes" WHERE "id" = $1"#),
            ("SELECT", Some("recipes"))
        );
        assert_eq!(
            target_of(r#"INSERT INTO "meals" ("id") VALUES ($1)"#),
            ("INSERT", Some("meals"))
        );
        assert_eq!(
            target_of(r#"UPDATE "users" SET "name" = $1"#),
            ("UPDATE", Some("users"))
        );
        assert_eq!(
            target_of(r#"DELETE FROM "sessions" WHERE "id" = $1"#),
            ("DELETE", Some("sessions"))
        );
    }

    #[test]
    fn target_has_no_table_for_other_statements() {
        assert_eq!(target_of("SET TIME ZONE 'UTC'"), ("SET", None));
        assert_eq!(target_of(""), ("query", None));
    }

    #[test]
    fn span_name_joins_operation_and_table() {
        assert_eq!(span_name("SELECT", Some("recipes")), "SELECT recipes");
        assert_eq!(span_name("SET", None), "SET");
    }
}

use {
    diesel::{
        deserialize::FromSql,
        pg::{Pg, PgValue},
        serialize::{IsNull, Output, ToSql},
        sql_types::Jsonb,
    },
    serde::{Serialize, de::DeserializeOwned},
    std::{fmt, io::Write},
    webauthn_rs::prelude::{Passkey, PasskeyAuthentication, PasskeyRegistration},
};

#[derive(Debug, diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct JsonWrapper<T>(pub T);

impl<T: Clone> From<&T> for JsonWrapper<T> {
    fn from(value: &T) -> Self {
        Self(value.to_owned())
    }
}

impl<T: DeserializeOwned> FromSql<Jsonb, Pg> for JsonWrapper<T> {
    fn from_sql(value: PgValue<'_>) -> diesel::deserialize::Result<Self> {
        // Copied from the diesel JsonB implementation.
        let bytes = value.as_bytes();
        if bytes[0] != 1 {
            return Err("Unsupported JSONB encoding version".into());
        }
        let value: T = serde_json::from_slice(&bytes[1..]).map_err(|_| "Invalid Json")?;
        Ok(JsonWrapper(value))
    }
}

impl<T: Serialize + fmt::Debug> ToSql<Jsonb, Pg> for JsonWrapper<T> {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        // Copied from the diesel JsonB implementation.
        out.write_all(&[1])?;
        serde_json::to_writer(out, &self.0)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

macro_rules! json_wrapper {
    () => {};
    ($ty:ident $($tail:tt)*) => {
        impl From<JsonWrapper<$ty>> for $ty {
            fn from(value: JsonWrapper<$ty>) -> $ty {
                value.0
            }
        }

        json_wrapper!($($tail)*);
    };
}

json_wrapper!(
    Passkey
    PasskeyAuthentication
    PasskeyRegistration
);

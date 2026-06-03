use {
    crate::error::ParseIdSnafu,
    serde::{Deserialize, Serialize, de},
    snafu::OptionExt,
    std::{
        fmt::{self, Write},
        hash::{Hash, Hasher},
        marker::PhantomData,
    },
    uuid::Uuid,
};

pub trait TablePrefix {
    const TABLE_PREFIX: &'static str;
}

#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Uuid))]
pub struct Id<T> {
    id: Uuid,
    _marker: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn from_uuid(id: Uuid) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.id
    }
}

impl<T: TablePrefix> Serialize for Id<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(s)
    }
}

impl<'de, T: TablePrefix> Deserialize<'de> for Id<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;

        s.parse()
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(&s), &"a uuid with a prefix"))
    }
}

impl<T> Id<T> {
    pub fn created_at(&self) -> jiff::Timestamp {
        let uuid_ts = self.id.get_timestamp().expect("we only use uuid v7");
        let (secs, nanos) = uuid_ts.to_unix();
        jiff::Timestamp::new(secs as i64, nanos as i32).expect("known good timestamp")
    }
}

macro_rules! table_id {
    () => {};
    ($id:ident, $draft_id:ident, $struct:ident, $prefix:tt; $($tail:tt)*) => {
        pub struct $struct;

        impl TablePrefix for $struct {
            const TABLE_PREFIX: &'static str = stringify!($prefix);
        }

        pub type $id = Id<$struct>;
        pub type $draft_id = DraftId<$struct>;

        table_id!($($tail)*);
    };
}

table_id! {
    BookId, BookDraftId, BookTable, bok;
    IngredientId, IngredientDraftId, IngredientTable, ing;
    MealId, MealDraftId, MealTable, mel;
    MealRecipeId, MealRecipeDraftId, MealRecipeTable, mrp;
    RecipeStepIngredientId, RecipeStepIngredientDraftId, RecipeStepIngredientTable, rsi;
    RecipeStepId, RecipeStepDraftId, RecipeStepTable, rst;
    RecipeId, RecipeDraftId, RecipeTable, rec;
    ShoppingListId, ShoppingListDraftId, ShoppingListTable, shl;
    ShoppingListItemId, ShoppingListItemDraftId, ShoppingListItemTable, sli;
    UserRoleId, UserRoleDraftId, UserRoleTable, url;
    UserId, UserDraftId, UserTable, usr;
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for Id<T> {}

impl<T: TablePrefix> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(T::TABLE_PREFIX)?;
        f.write_char('_')?;
        self.id.simple().fmt(f)
    }
}

impl<T: TablePrefix> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(T::TABLE_PREFIX)?;
        f.write_char('_')?;
        self.id.simple().fmt(f)
    }
}

impl<T: TablePrefix> std::str::FromStr for Id<T> {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid_str = s
            .strip_prefix(T::TABLE_PREFIX)
            .and_then(|s| s.strip_prefix('_'))
            .with_context(|| ParseIdSnafu { id: s.to_string() })?;

        Ok(Self {
            id: Uuid::try_parse(uuid_str)
                .ok()
                .with_context(|| ParseIdSnafu { id: s.to_string() })?,
            _marker: PhantomData,
        })
    }
}

#[cfg(feature = "server")]
impl<DB, T> diesel::deserialize::FromSql<diesel::sql_types::Uuid, DB> for Id<T>
where
    DB: diesel::backend::Backend,
    T: TablePrefix,
    Uuid: diesel::deserialize::FromSql<diesel::sql_types::Uuid, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let id = Uuid::from_sql(bytes)?;

        Ok(Self {
            id,
            _marker: PhantomData,
        })
    }
}

#[cfg(feature = "server")]
impl<DB, T> diesel::serialize::ToSql<diesel::sql_types::Uuid, DB> for Id<T>
where
    DB: diesel::backend::Backend,
    T: TablePrefix,
    Uuid: diesel::serialize::ToSql<diesel::sql_types::Uuid, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.id.to_sql(out)
    }
}

/// Stable identifier for a row inside an edit form, before and after it has
/// been saved.
///
/// `Persisted` wraps the real [`Id`] of a row that already exists in the
/// database; `New` is a provisional id allocated by the form itself, via a
/// deterministic per-form counter ([`DraftId::next`]) rather than any
/// process-global source, so server-rendered HTML matches what the client
/// produces on first render and Dioxus hydration succeeds.
///
/// It is generic over the same table marker as `Id<T>`, so a draft step id
/// can't be mixed up with a draft ingredient id. As a wire value it is the key
/// the server uses to decide insert (`New`) vs. update (`Persisted`) during an
/// upsert.
#[derive(Serialize, Deserialize)]
#[serde(untagged, bound = "")]
pub enum DraftId<T: TablePrefix> {
    Persisted(Id<T>),
    New(i64),
}

impl<T: TablePrefix> DraftId<T> {
    /// The next provisional id for a form, one past the highest `New` id already
    /// present. Deterministic given the same set of rows, so it produces the
    /// same value during SSR and hydration.
    pub fn next(existing: impl IntoIterator<Item = DraftId<T>>) -> DraftId<T> {
        let max = existing
            .into_iter()
            .filter_map(|d| match d {
                DraftId::New(n) => Some(n),
                DraftId::Persisted(_) => None,
            })
            .max()
            .unwrap_or(-1);

        DraftId::New(max + 1)
    }

    /// The real database id, if this row has been persisted.
    pub fn persisted(self) -> Option<Id<T>> {
        match self {
            DraftId::Persisted(id) => Some(id),
            DraftId::New(_) => None,
        }
    }
}

impl<T: TablePrefix> Default for DraftId<T> {
    fn default() -> Self {
        Self::New(0)
    }
}

impl<T: TablePrefix> From<Id<T>> for DraftId<T> {
    fn from(id: Id<T>) -> Self {
        Self::Persisted(id)
    }
}

impl<T: TablePrefix> Clone for DraftId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: TablePrefix> Copy for DraftId<T> {}

impl<T: TablePrefix> PartialEq for DraftId<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DraftId::Persisted(a), DraftId::Persisted(b)) => a == b,
            (DraftId::New(a), DraftId::New(b)) => a == b,
            _ => false,
        }
    }
}

impl<T: TablePrefix> Eq for DraftId<T> {}

impl<T: TablePrefix> Hash for DraftId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            DraftId::Persisted(id) => (0u8, id).hash(state),
            DraftId::New(n) => (1u8, n).hash(state),
        }
    }
}

impl<T: TablePrefix> fmt::Display for DraftId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DraftId::Persisted(id) => write!(f, "p-{id}"),
            DraftId::New(n) => write!(f, "n-{n}"),
        }
    }
}

impl<T: TablePrefix> fmt::Debug for DraftId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DraftId::Persisted(id) => write!(f, "Persisted({id})"),
            DraftId::New(n) => write!(f, "New({n})"),
        }
    }
}

use std::{
    fmt::{self, Write},
    hash::Hash,
    marker::PhantomData,
};

use uuid::Uuid;

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
    pub fn created_at(&self) -> jiff::Timestamp {
        let uuid_ts = self.id.get_timestamp().expect("we only use uuid v7");
        let (secs, nanos) = uuid_ts.to_unix();
        jiff::Timestamp::new(secs as i64, nanos as i32).expect("known good timestamp")
    }
}

macro_rules! table_id {
    () => {};
    ($id:ident, $struct:ident, $prefix:tt; $($tail:tt)*) => {
        pub struct $struct;

        impl TablePrefix for $struct {
            const TABLE_PREFIX: &'static str = stringify!(prefix);
        }

        pub type $id = Id<$struct>;

        table_id!($($tail)*);
    };
}

table_id! {
    BookId, BookTable, bok;
    IngredientId, IngredientTable, ing;
    MealId, MealTable, mel;
    MealRecipeId, MealRecipeTable, mrp;
    RecipeStepIngredientId, RecipeStepIngredientTable, rsi;
    RecipeStepId, RecipeStepTable, rst;
    RecipeId, RecipeTable, rec;
    ShoppingListId, ShoppingListTable, shl;
    ShoppingListItemId, ShoppingListItemTable, sli;
    UserRoleId, UserRoleTable, url;
    UserId, UserTable, usr;
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            _marker: self._marker.clone(),
        }
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
        self.id.fmt(f)
    }
}

impl<T: TablePrefix> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(T::TABLE_PREFIX)?;
        f.write_char('_')?;
        self.id.fmt(f)
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

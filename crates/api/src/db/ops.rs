use {
    super::{
        models,
        schema::{
            ingredients, meal_recipes, meals, recipe_step_ingredients, recipe_steps, recipes,
            shopping_list_items, shopping_lists, user_roles,
        },
    },
    anyhow::{Context, Result, anyhow},
    diesel::prelude::*,
    std::collections::HashMap,
    types::{
        GrocerySection, Ingredient, IngredientUpdate, Meal, MealDetail, MealRecipe, NewMeal,
        NewRecipe, NewShoppingList, NewShoppingListItem, NewStep, Recipe, RecipeDetail, RecipeStep,
        ShoppingList, ShoppingListDetail, Unit,
        id::{BookId, IngredientId, MealId, RecipeId, ShoppingListId, ShoppingListItemId, UserId},
        slugify,
    },
};

// ---------------------------------------------------------------------------
// Ingredient
// ---------------------------------------------------------------------------

impl models::Ingredient {
    pub fn list(conn: &mut PgConnection, book_id: BookId) -> Result<Vec<Ingredient>> {
        let rows = ingredients::table
            .filter(ingredients::book_id.eq(book_id))
            .order(ingredients::name.asc())
            .load::<models::Ingredient>(conn)
            .context("list_ingredients")?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub fn list_sections(
        conn: &mut PgConnection,
        book_id: BookId,
    ) -> Result<HashMap<IngredientId, Option<GrocerySection>>> {
        let rows: Vec<(IngredientId, Option<String>)> = ingredients::table
            .filter(ingredients::book_id.eq(book_id))
            .select((ingredients::id, ingredients::grocery_section))
            .load(conn)
            .context("list_ingredient_sections")?;

        rows.into_iter()
            .map(|(id, section)| {
                let gs = section
                    .as_deref()
                    .map(|s| s.parse::<GrocerySection>())
                    .transpose()
                    .with_context(|| format!("ingredient {id:?} has unknown grocery_section"))?;
                Ok((id, gs))
            })
            .collect()
    }

    pub fn update(
        conn: &mut PgConnection,
        book_id: BookId,
        id: IngredientId,
        input: IngredientUpdate,
    ) -> Result<Ingredient> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(anyhow!("ingredient name is required"));
        }
        if let Some(d) = input.density_g_per_ml
            && (!d.is_finite() || d <= 0.0)
        {
            return Err(anyhow!("density must be a positive number, got {d}"));
        }
        let section = input.grocery_section.map(|s| s.to_string());
        diesel::update(
            ingredients::table
                .filter(ingredients::id.eq(id))
                .filter(ingredients::book_id.eq(book_id)),
        )
        .set((
            ingredients::name.eq(name),
            ingredients::density_g_per_ml.eq(input.density_g_per_ml),
            ingredients::grocery_section.eq(section),
        ))
        .returning(models::Ingredient::as_returning())
        .get_result::<models::Ingredient>(conn)
        .optional()
        .context("update_ingredient")?
        .ok_or_else(|| anyhow!("ingredient {id:?} not found"))
        .map(Into::into)
    }
}

// ---------------------------------------------------------------------------
// Recipe
// ---------------------------------------------------------------------------

impl models::Recipe {
    pub fn list(conn: &mut PgConnection, book_id: BookId) -> Result<Vec<Recipe>> {
        let rows = recipes::table
            .filter(recipes::book_id.eq(book_id))
            .order(recipes::name.asc())
            .load::<models::Recipe>(conn)
            .context("list_recipes")?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub fn by_slug(
        conn: &mut PgConnection,
        book_id: BookId,
        slug: &str,
    ) -> Result<Option<RecipeDetail>> {
        recipes::table
            .filter(recipes::book_id.eq(book_id))
            .filter(recipes::slug.eq(slug))
            .first::<models::Recipe>(conn)
            .optional()
            .context("get_recipe by slug")?
            .map(|r| r.load_detail(conn))
            .transpose()
    }

    pub fn by_id(
        conn: &mut PgConnection,
        book_id: BookId,
        id: RecipeId,
    ) -> Result<Option<RecipeDetail>> {
        recipes::table
            .filter(recipes::book_id.eq(book_id))
            .filter(recipes::id.eq(id))
            .first::<models::Recipe>(conn)
            .optional()
            .context("get_recipe by id")?
            .map(|r| r.load_detail(conn))
            .transpose()
    }

    fn load_detail(self, conn: &mut PgConnection) -> Result<RecipeDetail> {
        let step_rows = models::RecipeStep::belonging_to(&self)
            .order(recipe_steps::position.asc())
            .load::<models::RecipeStep>(conn)
            .context("get_recipe steps")?;

        let ing_rows = models::RecipeStepIngredient::belonging_to(&step_rows)
            .inner_join(ingredients::table)
            .order(recipe_step_ingredients::position.asc())
            .load::<(models::RecipeStepIngredient, models::Ingredient)>(conn)
            .context("get_recipe step ingredients")?;

        let mut ings_by_step: HashMap<_, Vec<_>> = HashMap::new();
        for (rsi, ing) in ing_rows {
            ings_by_step
                .entry(rsi.step_id)
                .or_default()
                .push((rsi, ing));
        }

        let mut steps = Vec::with_capacity(step_rows.len());
        for sr in step_rows {
            let ingredients = ings_by_step
                .get(&sr.id)
                .into_iter()
                .flatten()
                .map(|(rsi, ing)| rsi.clone().into_type(ing.name.clone()))
                .collect::<Result<Vec<_>>>()?;
            steps.push(RecipeStep {
                id: sr.id,
                position: sr.position,
                text: sr.text,
                ingredients,
                duration_s: sr.duration_s,
            });
        }

        Ok(RecipeDetail {
            id: self.id,
            slug: self.slug,
            name: self.name,
            source: self.source,
            description: self.description,
            notes: self.notes,
            steps,
        })
    }

    pub fn create(conn: &mut PgConnection, book_id: BookId, input: NewRecipe) -> Result<String> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(anyhow!("recipe name is required"));
        }
        let source = input
            .source
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let description = input
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let notes = input
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let converted = convert_steps(&input.steps)?;

        conn.transaction(|conn| {
            let slug = unique_slug(conn, book_id, &slugify(name), |conn, slug| {
                diesel::select(diesel::dsl::exists(
                    recipes::table
                        .filter(recipes::book_id.eq(book_id))
                        .filter(recipes::slug.eq(slug)),
                ))
                .get_result(conn)
                .context("unique_recipe_slug probe")
            })?;

            let row = diesel::insert_into(recipes::table)
                .values(models::NewRecipe {
                    book_id,
                    slug: &slug,
                    name,
                    source,
                    description,
                    notes,
                })
                .get_result::<models::Recipe>(conn)
                .context("insert recipe")?;

            insert_steps(conn, book_id, row.id, &converted)?;
            Ok(slug)
        })
    }

    pub fn update(
        conn: &mut PgConnection,
        book_id: BookId,
        slug: &str,
        input: NewRecipe,
    ) -> Result<()> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(anyhow!("recipe name is required"));
        }
        let source = input
            .source
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let description = input
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let notes = input
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let converted = convert_steps(&input.steps)?;

        conn.transaction(|conn| {
            let row = diesel::update(
                recipes::table
                    .filter(recipes::book_id.eq(book_id))
                    .filter(recipes::slug.eq(slug)),
            )
            .set((
                recipes::name.eq(name),
                recipes::source.eq(source.unwrap_or("")),
                recipes::description.eq(description.unwrap_or("")),
                recipes::notes.eq(notes.unwrap_or("")),
            ))
            .get_result::<models::Recipe>(conn)
            .optional()
            .context("update recipe")?
            .ok_or_else(|| anyhow!("recipe `{slug}` not found"))?;

            diesel::delete(recipe_steps::table.filter(recipe_steps::recipe_id.eq(row.id)))
                .execute(conn)
                .context("delete old steps")?;

            insert_steps(conn, book_id, row.id, &converted)
        })
    }

    pub fn delete(conn: &mut PgConnection, book_id: BookId, slug: &str) -> Result<()> {
        let id = recipes::table
            .filter(recipes::book_id.eq(book_id))
            .filter(recipes::slug.eq(slug))
            .select(recipes::id)
            .first::<RecipeId>(conn)
            .optional()
            .context("recipe_id_for_slug")?
            .ok_or_else(|| anyhow!("recipe `{slug}` not found"))?;

        let blocking: Vec<String> = meal_recipes::table
            .filter(meal_recipes::recipe_id.eq(id))
            .inner_join(meals::table)
            .select(meals::name)
            .order(meals::name.asc())
            .load(conn)
            .context("check meal references")?;

        if !blocking.is_empty() {
            return Err(anyhow!(
                "recipe is used by {} meal(s): {}. Remove it from those meals first.",
                blocking.len(),
                blocking.join(", "),
            ));
        }

        let affected = diesel::delete(
            recipes::table
                .filter(recipes::book_id.eq(book_id))
                .filter(recipes::id.eq(id)),
        )
        .execute(conn)
        .context("delete recipe")?;

        if affected == 0 {
            return Err(anyhow!("recipe `{slug}` not found"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Meal
// ---------------------------------------------------------------------------

impl models::Meal {
    pub fn list(conn: &mut PgConnection, book_id: BookId) -> Result<Vec<Meal>> {
        let rows = meals::table
            .filter(meals::book_id.eq(book_id))
            .order(meals::name.asc())
            .load::<models::Meal>(conn)
            .context("list_meals")?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub fn by_slug(
        conn: &mut PgConnection,
        book_id: BookId,
        slug: &str,
    ) -> Result<Option<MealDetail>> {
        meals::table
            .filter(meals::book_id.eq(book_id))
            .filter(meals::slug.eq(slug))
            .first::<models::Meal>(conn)
            .optional()
            .context("get_meal by slug")?
            .map(|m| m.load_detail(conn, book_id))
            .transpose()
    }

    pub fn by_id(
        conn: &mut PgConnection,
        book_id: BookId,
        id: MealId,
    ) -> Result<Option<MealDetail>> {
        meals::table
            .filter(meals::book_id.eq(book_id))
            .filter(meals::id.eq(id))
            .first::<models::Meal>(conn)
            .optional()
            .context("get_meal by id")?
            .map(|m| m.load_detail(conn, book_id))
            .transpose()
    }

    fn load_detail(self, conn: &mut PgConnection, book_id: BookId) -> Result<MealDetail> {
        let mr_rows = models::MealRecipe::belonging_to(&self)
            .order(meal_recipes::position.asc())
            .load::<models::MealRecipe>(conn)
            .context("get_meal recipes")?;

        let mut recipe_list = Vec::with_capacity(mr_rows.len());
        for mr in mr_rows {
            let detail = models::Recipe::by_id(conn, book_id, mr.recipe_id)?.ok_or_else(|| {
                anyhow!(
                    "meal {:?} references missing recipe {:?}",
                    self.id,
                    mr.recipe_id
                )
            })?;
            recipe_list.push(MealRecipe {
                id: mr.id,
                multiplier: mr.multiplier,
                position: mr.position,
                recipe: detail,
            });
        }

        Ok(MealDetail {
            id: self.id,
            slug: self.slug,
            name: self.name,
            recipes: recipe_list,
        })
    }

    pub fn create(conn: &mut PgConnection, book_id: BookId, input: NewMeal) -> Result<String> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(anyhow!("meal name is required"));
        }
        validate_meal_recipes(&input.recipes)?;

        conn.transaction(|conn| {
            let base = slugify(name);
            let base = base.strip_prefix("local-").unwrap_or(&base);
            let base = if base.is_empty() { "meal" } else { base };
            let slug = unique_slug(conn, book_id, base, |conn, slug| {
                diesel::select(diesel::dsl::exists(
                    meals::table
                        .filter(meals::book_id.eq(book_id))
                        .filter(meals::slug.eq(slug)),
                ))
                .get_result(conn)
                .context("unique_meal_slug probe")
            })?;

            let row = diesel::insert_into(meals::table)
                .values(models::NewMeal {
                    book_id,
                    slug: &slug,
                    name,
                })
                .get_result::<models::Meal>(conn)
                .context("insert meal")?;

            insert_meal_recipes(conn, book_id, row.id, &input.recipes)?;
            Ok(slug)
        })
    }

    pub fn update(
        conn: &mut PgConnection,
        book_id: BookId,
        slug: &str,
        input: NewMeal,
    ) -> Result<()> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(anyhow!("meal name is required"));
        }
        validate_meal_recipes(&input.recipes)?;

        conn.transaction(|conn| {
            let row = diesel::update(
                meals::table
                    .filter(meals::book_id.eq(book_id))
                    .filter(meals::slug.eq(slug)),
            )
            .set(meals::name.eq(name))
            .get_result::<models::Meal>(conn)
            .optional()
            .context("update meal")?
            .ok_or_else(|| anyhow!("meal `{slug}` not found"))?;

            diesel::delete(meal_recipes::table.filter(meal_recipes::meal_id.eq(row.id)))
                .execute(conn)
                .context("clear meal recipes")?;

            insert_meal_recipes(conn, book_id, row.id, &input.recipes)
        })
    }

    pub fn delete(conn: &mut PgConnection, book_id: BookId, slug: &str) -> Result<()> {
        let affected = diesel::delete(
            meals::table
                .filter(meals::book_id.eq(book_id))
                .filter(meals::slug.eq(slug)),
        )
        .execute(conn)
        .context("delete meal")?;

        if affected == 0 {
            return Err(anyhow!("meal `{slug}` not found"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ShoppingList
// ---------------------------------------------------------------------------

impl models::ShoppingList {
    pub fn list(conn: &mut PgConnection, book_id: BookId) -> Result<Vec<ShoppingList>> {
        let rows = shopping_lists::table
            .filter(shopping_lists::book_id.eq(book_id))
            .order(shopping_lists::name.asc())
            .load::<models::ShoppingList>(conn)
            .context("list_shopping_lists")?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub fn get(
        conn: &mut PgConnection,
        book_id: BookId,
        id: ShoppingListId,
    ) -> Result<Option<ShoppingListDetail>> {
        shopping_lists::table
            .filter(shopping_lists::book_id.eq(book_id))
            .filter(shopping_lists::id.eq(id))
            .first::<models::ShoppingList>(conn)
            .optional()
            .context("get_shopping_list")?
            .map(|list| list.load_detail(conn))
            .transpose()
    }

    fn load_detail(self, conn: &mut PgConnection) -> Result<ShoppingListDetail> {
        let item_rows = models::ShoppingListItem::belonging_to(&self)
            .order(shopping_list_items::position.asc())
            .load::<models::ShoppingListItem>(conn)
            .context("get_shopping_list items")?;

        let ing_ids: Vec<IngredientId> = item_rows.iter().filter_map(|i| i.ingredient_id).collect();

        let ingredient_map: HashMap<IngredientId, models::Ingredient> = if ing_ids.is_empty() {
            HashMap::new()
        } else {
            ingredients::table
                .filter(ingredients::id.eq_any(&ing_ids))
                .load::<models::Ingredient>(conn)
                .context("get_shopping_list ingredients")?
                .into_iter()
                .map(|i| (i.id, i))
                .collect()
        };

        let items = item_rows
            .into_iter()
            .map(|item| {
                let ing = item.ingredient_id.and_then(|id| ingredient_map.get(&id));
                item.into_type(ing)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ShoppingListDetail {
            id: self.id,
            slug: self.slug,
            name: self.name,
            items,
        })
    }

    pub fn create(
        conn: &mut PgConnection,
        book_id: BookId,
        input: NewShoppingList,
    ) -> Result<ShoppingListId> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(anyhow!("shopping list name is required"));
        }

        conn.transaction(|conn| {
            let slug = unique_slug(conn, book_id, &slugify(name), |conn, slug| {
                diesel::select(diesel::dsl::exists(
                    shopping_lists::table
                        .filter(shopping_lists::book_id.eq(book_id))
                        .filter(shopping_lists::slug.eq(slug)),
                ))
                .get_result(conn)
                .context("unique_list_slug probe")
            })?;

            let row = diesel::insert_into(shopping_lists::table)
                .values(models::NewShoppingList {
                    book_id,
                    slug: &slug,
                    name,
                })
                .get_result::<models::ShoppingList>(conn)
                .context("insert shopping_list")?;

            for (idx, item) in input.items.iter().enumerate() {
                models::ShoppingListItem::insert_new(conn, book_id, row.id, item, idx as i32)?;
            }
            Ok(row.id)
        })
    }

    pub fn delete(conn: &mut PgConnection, book_id: BookId, id: ShoppingListId) -> Result<()> {
        let affected = diesel::delete(
            shopping_lists::table
                .filter(shopping_lists::book_id.eq(book_id))
                .filter(shopping_lists::id.eq(id)),
        )
        .execute(conn)
        .context("delete shopping_list")?;

        if affected == 0 {
            return Err(anyhow!("shopping list {id:?} not found"));
        }
        Ok(())
    }
}

impl models::ShoppingListItem {
    pub fn add(
        conn: &mut PgConnection,
        book_id: BookId,
        list_id: ShoppingListId,
        item: &NewShoppingListItem,
    ) -> Result<ShoppingListItemId> {
        let next_pos: i32 = shopping_list_items::table
            .filter(shopping_list_items::shopping_list_id.eq(list_id))
            .select(diesel::dsl::max(shopping_list_items::position))
            .first::<Option<i32>>(conn)
            .context("next item position")?
            .map(|p| p + 1)
            .unwrap_or(0);

        Self::insert_new(conn, book_id, list_id, item, next_pos)
    }

    pub(crate) fn insert_new(
        conn: &mut PgConnection,
        book_id: BookId,
        list_id: ShoppingListId,
        item: &NewShoppingListItem,
        position: i32,
    ) -> Result<ShoppingListItemId> {
        let unit_kind = item.unit.as_ref().map(|u| u.kind().to_string());
        let unit_label = item.unit.as_ref().map(|u| u.label());
        diesel::insert_into(shopping_list_items::table)
            .values(models::NewShoppingListItem {
                book_id,
                shopping_list_id: list_id,
                position,
                quantity: item.quantity,
                unit_kind: unit_kind.as_deref(),
                unit: unit_label.as_deref(),
                ingredient_id: item.ingredient_id,
                text: item.text.as_deref(),
            })
            .get_result::<models::ShoppingListItem>(conn)
            .context("insert shopping_list_item")
            .map(|row| row.id)
    }

    pub fn set_checked(
        conn: &mut PgConnection,
        book_id: BookId,
        item_id: ShoppingListItemId,
        checked: bool,
    ) -> Result<()> {
        let affected = diesel::update(
            shopping_list_items::table
                .filter(shopping_list_items::book_id.eq(book_id))
                .filter(shopping_list_items::id.eq(item_id)),
        )
        .set(shopping_list_items::checked.eq(checked))
        .execute(conn)
        .context("set_checked shopping_list_item")?;

        if affected == 0 {
            return Err(anyhow!("shopping list item {item_id:?} not found"));
        }
        Ok(())
    }

    pub fn delete(
        conn: &mut PgConnection,
        book_id: BookId,
        item_id: ShoppingListItemId,
    ) -> Result<()> {
        let affected = diesel::delete(
            shopping_list_items::table
                .filter(shopping_list_items::book_id.eq(book_id))
                .filter(shopping_list_items::id.eq(item_id)),
        )
        .execute(conn)
        .context("delete shopping_list_item")?;

        if affected == 0 {
            return Err(anyhow!("shopping list item {item_id:?} not found"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// User / auth helpers
// ---------------------------------------------------------------------------

impl models::User {
    pub fn load_for_book(
        conn: &mut PgConnection,
        user_id: UserId,
        book_id: BookId,
    ) -> Result<Option<(models::User, models::Role)>> {
        let row: Option<(models::User, models::UserRole)> = super::schema::users::table
            .inner_join(user_roles::table)
            .filter(super::schema::users::id.eq(user_id))
            .filter(user_roles::book_id.eq(book_id))
            .first(conn)
            .optional()
            .context("load_user_for_book")?;

        Ok(row.map(|(u, ur)| (u, ur.role)))
    }

    pub fn list_all(conn: &mut PgConnection) -> Result<Vec<models::User>> {
        super::schema::users::table
            .order(super::schema::users::name.asc())
            .load(conn)
            .context("list_users")
    }

    pub fn upsert_oidc(
        conn: &mut PgConnection,
        _sub: &str,
        email: &str,
        name: &str,
    ) -> Result<(UserId, BookId)> {
        use super::schema::users;

        let user: models::User = diesel::insert_into(users::table)
            .values((users::email.eq(email), users::name.eq(name)))
            .on_conflict(users::email)
            .do_update()
            .set(users::name.eq(name))
            .get_result(conn)
            .context("upsert_user")?;

        let book_id = ensure_user_book(conn, user.id, name)?;
        Ok((user.id, book_id))
    }
}

fn ensure_user_book(
    conn: &mut PgConnection,
    user_id: UserId,
    display_name: &str,
) -> Result<BookId> {
    use super::schema::{books, user_roles as ur};

    if let Some(book_id) = ur::table
        .filter(ur::user_id.eq(user_id))
        .select(ur::book_id)
        .first::<BookId>(conn)
        .optional()
        .context("lookup user book")?
    {
        return Ok(book_id);
    }

    let slug = slugify(display_name);
    let book: models::Book = diesel::insert_into(books::table)
        .values(models::NewBook {
            name: display_name,
            slug: &slug,
            owner_id: user_id,
        })
        .get_result(conn)
        .context("create user book")?;

    diesel::insert_into(user_roles::table)
        .values(models::NewUserRole {
            book_id: book.id,
            user_id,
            role: models::Role::Admin,
        })
        .execute(conn)
        .context("create user_role")?;

    Ok(book.id)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn unique_slug(
    conn: &mut PgConnection,
    _book_id: BookId,
    base: &str,
    exists: impl Fn(&mut PgConnection, &str) -> Result<bool>,
) -> Result<String> {
    let mut candidate = base.to_string();
    let mut n: u32 = 2;
    loop {
        if !exists(conn, &candidate)? {
            return Ok(candidate);
        }
        candidate = format!("{base}-{n}");
        n = n
            .checked_add(1)
            .ok_or_else(|| anyhow!("slug space exhausted"))?;
    }
}

struct ConvertedStep {
    text: String,
    ingredients: Vec<ConvertedIngredient>,
    duration_s: Option<i32>,
}

struct ConvertedIngredient {
    name: String,
    quantity: Option<f64>,
    unit: Option<Unit>,
}

fn convert_steps(steps: &[NewStep]) -> Result<Vec<ConvertedStep>> {
    let mut out = Vec::with_capacity(steps.len());
    for (step_idx, step) in steps.iter().enumerate() {
        let mut ings = Vec::with_capacity(step.ingredients.len());
        for (ing_idx, ing) in step.ingredients.iter().enumerate() {
            let name = ing.ingredient_name.trim();
            if name.is_empty() {
                continue;
            }
            if let Some(q) = ing.quantity
                && (!q.is_finite() || q < 0.0)
            {
                return Err(anyhow!(
                    "step {} ingredient {} ({name}): quantity must be non-negative, got {}",
                    step_idx + 1,
                    ing_idx + 1,
                    q,
                ));
            }
            let unit = match ing.unit_kind {
                Some(kind) => Some(Unit::new(kind, &ing.unit).map_err(|e| {
                    anyhow!(
                        "step {} ingredient {} ({name}): {e}",
                        step_idx + 1,
                        ing_idx + 1
                    )
                })?),
                None => None,
            };
            ings.push(ConvertedIngredient {
                name: name.to_string(),
                quantity: ing.quantity,
                unit,
            });
        }

        if let Some(d) = step.duration_s
            && d <= 0
        {
            return Err(anyhow!(
                "step {}: duration must be positive, got {d}s",
                step_idx + 1
            ));
        }

        out.push(ConvertedStep {
            text: step.text.clone(),
            ingredients: ings,
            duration_s: step.duration_s,
        });
    }
    Ok(out)
}

fn insert_steps(
    conn: &mut PgConnection,
    book_id: BookId,
    recipe_id: RecipeId,
    steps: &[ConvertedStep],
) -> Result<()> {
    for (idx, step) in steps.iter().enumerate() {
        let step_row = diesel::insert_into(recipe_steps::table)
            .values(models::NewRecipeStep {
                book_id,
                recipe_id,
                position: idx as i32,
                text: &step.text,
                duration_s: step.duration_s,
            })
            .get_result::<models::RecipeStep>(conn)
            .context("insert step")?;

        for (ing_idx, ing) in step.ingredients.iter().enumerate() {
            let ingredient_id = upsert_ingredient_by_name(conn, book_id, &ing.name)?;
            let unit_kind = ing.unit.as_ref().map(|u| u.kind().to_string());
            let unit_label = ing.unit.as_ref().map(|u| u.label());
            diesel::insert_into(recipe_step_ingredients::table)
                .values(models::NewRecipeStepIngredient {
                    book_id,
                    step_id: step_row.id,
                    position: ing_idx as i32,
                    quantity: ing.quantity,
                    unit_kind: unit_kind.as_deref(),
                    unit: unit_label.as_deref(),
                    ingredient_id,
                })
                .execute(conn)
                .context("insert step ingredient")?;
        }
    }
    Ok(())
}

fn upsert_ingredient_by_name(
    conn: &mut PgConnection,
    book_id: BookId,
    name: &str,
) -> Result<IngredientId> {
    let name_lower = name.to_lowercase();
    let existing: Option<IngredientId> = ingredients::table
        .filter(ingredients::book_id.eq(book_id))
        .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
            "lower(name) = '{}'",
            name_lower.replace('\'', "''")
        )))
        .select(ingredients::id)
        .first(conn)
        .optional()
        .context("lookup ingredient by name")?;

    if let Some(id) = existing {
        return Ok(id);
    }

    diesel::insert_into(ingredients::table)
        .values(models::NewIngredient {
            book_id,
            name,
            density_g_per_ml: None,
            grocery_section: None,
        })
        .get_result::<models::Ingredient>(conn)
        .context("insert ingredient")
        .map(|row| row.id)
}

fn validate_meal_recipes(recipes: &[types::NewMealRecipe]) -> Result<()> {
    let mut seen = std::collections::HashSet::with_capacity(recipes.len());
    for (idx, mr) in recipes.iter().enumerate() {
        if !mr.multiplier.is_finite() || mr.multiplier <= 0.0 {
            return Err(anyhow!(
                "recipe {} multiplier must be positive, got {}",
                idx + 1,
                mr.multiplier
            ));
        }
        if !seen.insert(&mr.recipe_slug) {
            return Err(anyhow!(
                "recipe {} (`{}`) appears more than once",
                idx + 1,
                mr.recipe_slug,
            ));
        }
    }
    Ok(())
}

fn insert_meal_recipes(
    conn: &mut PgConnection,
    book_id: BookId,
    meal_id: MealId,
    recipes: &[types::NewMealRecipe],
) -> Result<()> {
    for (idx, mr) in recipes.iter().enumerate() {
        let recipe_id = recipes::table
            .filter(recipes::book_id.eq(book_id))
            .filter(recipes::slug.eq(&mr.recipe_slug))
            .select(recipes::id)
            .first::<RecipeId>(conn)
            .optional()
            .context("lookup recipe for meal")?
            .ok_or_else(|| anyhow!("recipe `{}` not found", mr.recipe_slug))?;

        diesel::insert_into(meal_recipes::table)
            .values(models::NewMealRecipe {
                book_id,
                meal_id,
                recipe_id,
                multiplier: mr.multiplier,
                position: idx as i32,
            })
            .execute(conn)
            .with_context(|| format!("insert meal_recipe (recipe `{}`)", mr.recipe_slug))?;
    }
    Ok(())
}

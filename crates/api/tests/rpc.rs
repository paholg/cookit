//! End-to-end coverage for the generated CRUD/sync layer (`db::rpc`): the
//! `Operation` batch endpoint, the tri-state `IngredientUpdate`, soft-delete,
//! per-table `list_since`, and the wire format of the partial-update enum.

use {
    crate::test_support::{TestBook, unique},
    api::{
        IngredientCreate, IngredientDelete, IngredientResponse, IngredientUpdate, Name, Operation,
        OperationResponse, PositiveFloat, apply_ops, grocery_section::GrocerySection,
        id::IngredientId, list_ingredients_since, me, update_ingredient,
    },
    db::Timestamp,
    uuid::Uuid,
};

mod test_support;

/// A sync watermark a few seconds in the past — recent enough that only rows
/// this test just wrote fall after it (so they land on the first page),
/// generous enough to absorb any app/DB clock skew.
fn recent_watermark() -> Timestamp {
    Timestamp::new(jiff::Timestamp::now() - jiff::SignedDuration::from_secs(5))
}

fn unwrap_ingredient(resp: OperationResponse) -> IngredientResponse {
    match resp {
        OperationResponse::Ingredient(row) => row,
        other => panic!("expected an ingredient response, got {other:?}"),
    }
}

async fn session_book() -> db::id::BookId {
    me().await.expect("me").expect("session").book_id
}

/// Create → list_since sees it → soft-delete → list_since still reports it with
/// `deleted_at` set (the deletion propagates through the timestamp sync).
#[tokio::test]
async fn ingredient_create_and_delete_sync() {
    TestBook::new().await;
    let book_id = session_book().await;

    let id = IngredientId::from_uuid(Uuid::now_v7());
    let watermark = recent_watermark();

    let create = IngredientCreate {
        id,
        book_id,
        name: Name::try_new(unique("rpc-sugar")).unwrap(),
        density_g_per_ml: Some(PositiveFloat::try_new(1.5).unwrap()),
        grocery_section: None,
    };

    let created = unwrap_ingredient(
        apply_ops(vec![Operation::IngredientCreate(create)])
            .await
            .expect("create")
            .into_iter()
            .next()
            .expect("one response"),
    );
    assert_eq!(created.id, id);
    assert_eq!(
        created.density_g_per_ml,
        Some(PositiveFloat::try_new(1.5).unwrap())
    );
    assert!(created.deleted_at.is_none());

    let page = list_ingredients_since(watermark).await.expect("since");
    assert!(
        page.records
            .iter()
            .any(|r| r.id == id && r.deleted_at.is_none()),
        "freshly created ingredient should sync"
    );

    let deleted = unwrap_ingredient(
        apply_ops(vec![Operation::IngredientDelete(IngredientDelete { id })])
            .await
            .expect("delete")
            .into_iter()
            .next()
            .expect("one response"),
    );
    assert_eq!(deleted.id, id);
    assert!(deleted.deleted_at.is_some(), "delete should be soft");

    let page = list_ingredients_since(watermark)
        .await
        .expect("since after delete");
    let row = page
        .records
        .iter()
        .find(|r| r.id == id)
        .expect("soft-deleted row still syncs");
    assert!(
        row.deleted_at.is_some(),
        "sync must carry the deletion to clients"
    );
}

/// The three-way semantics of `IngredientUpdate`: `Some(_)` sets, `None` leaves
/// a field untouched, and the nullable columns' inner `Option` can clear to NULL.
#[tokio::test]
async fn ingredient_update_is_tri_state() {
    TestBook::new().await;
    let book_id = session_book().await;

    let id = IngredientId::from_uuid(Uuid::now_v7());
    let original = unique("rpc-tri");

    apply_ops(vec![Operation::IngredientCreate(IngredientCreate {
        id,
        book_id,
        name: Name::try_new(&original).unwrap(),
        density_g_per_ml: Some(PositiveFloat::try_new(2.0).unwrap()),
        grocery_section: None,
    })])
    .await
    .expect("create");

    // Rename, leave density untouched, set the section.
    let renamed = unique("rpc-tri-renamed");
    let updated = update_ingredient(IngredientUpdate {
        id,
        name: Some(Name::try_new(&renamed).unwrap()),
        density_g_per_ml: None,
        grocery_section: Some(Some(GrocerySection::Dairy)),
    })
    .await
    .expect("update 1");
    assert_eq!(updated.name.as_ref(), renamed);
    assert_eq!(
        updated.density_g_per_ml,
        Some(PositiveFloat::try_new(2.0).unwrap()),
        "density left unchanged"
    );
    assert_eq!(updated.grocery_section, Some(GrocerySection::Dairy));

    // Clear density to NULL, leave name and section untouched.
    let updated = update_ingredient(IngredientUpdate {
        id,
        name: None,
        density_g_per_ml: Some(None),
        grocery_section: None,
    })
    .await
    .expect("update 2");
    assert!(
        updated.density_g_per_ml.is_none(),
        "density cleared to NULL"
    );
    assert_eq!(updated.name.as_ref(), renamed, "name left unchanged");
    assert_eq!(
        updated.grocery_section,
        Some(GrocerySection::Dairy),
        "section left unchanged"
    );
}

/// The wire format must distinguish "leave unchanged" (field omitted) from "set
/// NULL" (field present as `null`), or a partial update silently nulls columns.
#[test]
fn operation_update_wire_format() {
    let id = IngredientId::from_uuid(Uuid::now_v7());

    let leave = Operation::IngredientUpdate(IngredientUpdate {
        id,
        name: None,
        density_g_per_ml: None,
        grocery_section: None,
    });
    let json = serde_json::to_string(&leave).unwrap();
    assert!(
        !json.contains("density_g_per_ml"),
        "untouched fields must be omitted, got: {json}"
    );
    assert_eq!(serde_json::from_str::<Operation>(&json).unwrap(), leave);

    let clear = Operation::IngredientUpdate(IngredientUpdate {
        id,
        name: None,
        density_g_per_ml: Some(None),
        grocery_section: None,
    });
    let json = serde_json::to_string(&clear).unwrap();
    assert!(
        json.contains("\"density_g_per_ml\":null"),
        "set-NULL must serialize as null, got: {json}"
    );
    assert_eq!(serde_json::from_str::<Operation>(&json).unwrap(), clear);
}

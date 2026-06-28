#[cfg(feature = "server")]
use server::RequestContext;
use {
    db::{
        Email, Name,
        id::{UserId, UserPasskeyId},
        models::{passkey::PasskeyInfo, user::Current},
    },
    dioxus::prelude::*,
    webauthn_rs_proto::{
        CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
        RequestChallengeResponse,
    },
};

#[post("/api/auth/create-user", mut ctx: RequestContext)]
pub async fn create_user(name: Name, email: Email) -> Result<Current, ServerFnError> {
    let user = server::user::create(ctx.conn(), name, email).await?;

    let current = ctx.login_as(user).await?;

    Ok(current)
}

#[post("/api/register_passkey/start", mut ctx: RequestContext)]
pub async fn register_start() -> Result<CreationChallengeResponse, ServerFnError> {
    let user_id = ctx.require_user()?.id;

    let ccr = server::webauthn::client()
        .start_registration(ctx.conn(), user_id)
        .await?;

    Ok(ccr)
}

#[post("/api/register_passkey/finish", mut ctx: RequestContext)]
pub async fn register_finish(reg: RegisterPublicKeyCredential) -> Result<(), ServerFnError> {
    let user_id = ctx.require_user()?.id;

    server::webauthn::client()
        .finish_registration(ctx.conn(), user_id, &reg)
        .await?;

    Ok(())
}

#[post("/api/authenticate_passkey/start", mut ctx: RequestContext)]
pub async fn authenticate_start(
    email: Email,
) -> Result<(UserId, RequestChallengeResponse), ServerFnError> {
    use server::user;

    let conn = ctx.conn();
    let user = user::find_by_email(conn, &email).await?;

    let ccr = server::webauthn::client()
        .start_passkey_authentication(conn, user.id)
        .await?;

    Ok((user.id, ccr))
}

#[post("/api/authenticate_passkey/finish", mut ctx: RequestContext)]
pub async fn authenticate_finish(
    user_id: UserId,
    reg: PublicKeyCredential,
) -> Result<Current, ServerFnError> {
    use server::user;

    server::webauthn::client()
        .finish_passkey_authentication(ctx.conn(), user_id, &reg)
        .await?;

    let user = user::find(ctx.conn(), user_id).await?;
    let current = ctx.login_as(user).await?;

    Ok(current)
}

#[get("/api/passkeys", mut ctx: RequestContext)]
pub async fn list_passkeys() -> Result<Vec<PasskeyInfo>, ServerFnError> {
    let user_id = ctx.require_user()?.id;

    let passkeys = server::webauthn::list_passkeys(ctx.conn(), user_id).await?;

    Ok(passkeys)
}

#[post("/api/passkeys/delete", mut ctx: RequestContext)]
pub async fn delete_passkey(id: UserPasskeyId) -> Result<(), ServerFnError> {
    let user_id = ctx.require_user()?.id;

    server::webauthn::delete_passkey(ctx.conn(), user_id, id).await?;

    Ok(())
}

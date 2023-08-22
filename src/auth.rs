use std::sync::Arc;

use axum::extract::State;
use axum::extract::TypedHeader;
use axum::headers::Cookie;
use axum::http::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use constant_time_eq::constant_time_eq;
use django_signing::Signer;
use hmac::Hmac;
use hmac::Mac;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::app_error::AppError;
use crate::app_state::AppState;
use crate::types::{RealmId, UserId};

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user_id: UserId,
    pub realm_id: RealmId,
}

#[derive(Debug, Deserialize)]
struct Session {
    _auth_user_id: String,
    _auth_user_backend: String,
    _auth_user_hash: String,
}

async fn authenticate_django_session(
    state: &AppState,
    cookie: Cookie,
) -> Result<Option<AuthContext>, anyhow::Error> {
    // Get the sessionid cookie
    let Some(session_id) = cookie.get("sessionid") else {
        tracing::debug!("No session cookie");
        return Ok(None);
    };

    // Load the session from the database
    let db = state.db_pool.get().await?;
    let sql =
        "SELECT session_data FROM django_session WHERE session_key = $1 AND expire_date > now()";
    let Some(session_row) = db.query_opt(sql, &[&session_id]).await? else {
        tracing::debug!("No session row");
        return Ok(None);
    };
    let session_data: String = session_row.get("session_data");

    // Decode the session data
    let Ok(session) = django_signing::TimestampSigner::new(
        state.secret_key.as_bytes(),
        b"django.contrib.sessions.SessionStore",
    )
    .unsign_object::<Session>(session_data) else {
        tracing::debug!("Bad session data");
        return Ok(None);
    };

    let user_id: UserId = session._auth_user_id.parse()?;

    // Load the user from the database
    let sql = "SELECT password, realm_id FROM zerver_userprofile WHERE id = $1";
    let Some(user_row) = db.query_opt(sql, &[&user_id]).await? else {
        tracing::debug!("No user row");
        return Ok(None);
    };
    let password: &str = user_row.get("password");
    let realm_id: RealmId = user_row.get("realm_id");
    // TODO/boq: verify the realm against the request hostname

    // Verify the session hash
    let mut key_hash = Sha256::new();
    key_hash.update(b"django.contrib.auth.models.AbstractBaseUser.get_session_auth_hash");
    key_hash.update(state.secret_key.as_bytes());
    let mut mac: Hmac<Sha256> = Hmac::new_from_slice(&key_hash.finalize())?;
    mac.update(password.as_bytes());
    if !constant_time_eq(
        &hex::decode(session._auth_user_hash)?,
        &mac.finalize().into_bytes(),
    ) {
        tracing::debug!("Bad session hash");
        return Ok(None);
    }

    // User is authenticated
    Ok(Some(AuthContext { user_id, realm_id }))
}

pub async fn django_session_auth<B>(
    State(state): State<Arc<AppState>>,
    TypedHeader(cookie): TypedHeader<Cookie>,
    mut req: Request<B>,
) -> Result<Request<B>, Result<Response, AppError>> {
    if let Some(auth_context) = authenticate_django_session(&state, cookie)
        .await
        .map_err(|err| Err(err.into()))?
    {
        req.extensions_mut().insert(auth_context);
        Ok(req)
    } else {
        Err(Ok(StatusCode::UNAUTHORIZED.into_response()))
    }
}

pub async fn api_auth<B>(
    State(_state): State<Arc<AppState>>,
    TypedHeader(_cookie): TypedHeader<Cookie>,
    _req: Request<B>,
) -> Result<Request<B>, Result<Response, AppError>> {
    // TODO/boq: API authentication
    Err(Ok(StatusCode::UNAUTHORIZED.into_response()))
}

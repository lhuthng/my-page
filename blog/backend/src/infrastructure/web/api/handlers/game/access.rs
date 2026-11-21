// Who may play an 'invited' game. One resolver answers for every serving
// route and the page: the author and staff always can, a registered redeemer
// can through an account-bound grant, and a guest can through a signed cookie
// token minted at redemption. The guest token is deliberately stateless - no
// server row holds it - but its check re-reads the invitation row, so
// revoking a code takes a guest's access away on their next request.
use axum_extra::extract::cookie::{Cookie, CookieJar};
use jsonwebtoken::{Validation, decode};
use serde::{Deserialize, Serialize};

use crate::domain::entities::secret::Claims;
use crate::domain::errors::game::GameError;
use crate::infrastructure::web::server::AppState;

/// How long a guest's redeemed access lives, in the cookie and in the token's
/// `exp`. A single-use code must not silently expire out from under the
/// browser that burned it, so this is long; revoking the invitation is the
/// control that actually ends access, not an expiry.
const INVITE_ACCESS_DAYS: i64 = 365;

/// What the invite cookie carries. Small on purpose: the game it opens and
/// the code that opened it.
#[derive(Debug, Serialize, Deserialize)]
pub struct InviteClaims {
    pub game_id: i64,
    pub inv: i64,
    pub exp: usize,
}

/// The access facts one game row carries, fetched once per request.
#[derive(Debug, sqlx::FromRow)]
pub struct GameAccess {
    pub game_id: i64,
    pub access_mode: String,
    pub owner_user_id: i64,
}

/// Resolve by slug for every slug-addressed serving route. Published posts
/// only, matching the serving queries this guards.
pub(crate) async fn game_access_by_slug(
    pool: &sqlx::SqlitePool,
    slug: &str,
) -> Result<Option<GameAccess>, sqlx::Error> {
    sqlx::query_as(
        r#"SELECT games.id AS game_id, games.access_mode, posts.user_id AS owner_user_id
           FROM games
           JOIN posts ON posts.id = games.post_id
           WHERE posts.slug = ? AND posts.status = 'published' AND posts.deleted_at IS NULL"#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub fn invite_cookie_name(game_id: i64) -> String {
    format!("game-invite-{game_id}")
}

/// The one wire value authors may send for a game's access mode. Absent stays
/// 'open'; anything else is a validation error.
pub(crate) fn normalize_access_mode(raw: Option<&str>) -> Result<String, GameError> {
    match raw.map(str::trim) {
        None | Some("") | Some("open") => Ok("open".to_string()),
        Some("invited") => Ok("invited".to_string()),
        Some(other) => Err(GameError::InvalidDemo(format!(
            "Unknown access mode '{other}' (use 'open' or 'invited')."
        ))),
    }
}

pub(super) fn invite_cookie(game_id: i64, token: String) -> Cookie<'static> {
    Cookie::build((invite_cookie_name(game_id), token))
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::days(INVITE_ACCESS_DAYS))
        .build()
}

pub(super) async fn encode_invite_token(
    state: &AppState,
    game_id: i64,
    invitation_id: i64,
) -> Result<String, GameError> {
    let auth = &state.config.auth;
    let claims = InviteClaims {
        game_id,
        inv: invitation_id,
        exp: chrono::Utc::now()
            .checked_add_signed(chrono::Duration::days(INVITE_ACCESS_DAYS))
            .expect("valid timestamp")
            .timestamp() as usize,
    };
    jsonwebtoken::encode(&auth.header, &claims, &auth.encoding_key)
        .map_err(|_| GameError::InternalError("Invite token creation".to_string()))
}

/// `None` for anything unsigned, foreign, or expired - a bad cookie is the
/// same as no cookie, never an error.
async fn decode_invite_token(state: &AppState, token: String) -> Option<InviteClaims> {
    let auth = &state.config.auth;
    decode::<InviteClaims>(token, &auth.decoding_key, &Validation::new(auth.algorithm))
        .ok()
        .map(|data| data.claims)
}

/// The decision for one request: may this viewer play this game? `access`
/// comes from [`game_access_by_slug`]; `viewer` is the optional-auth claims
/// and `jar` the request's cookies. Open games are always true - the checks
/// below only exist for 'invited' ones.
pub(crate) async fn entitled(
    state: &AppState,
    access: &GameAccess,
    viewer: Option<&Claims>,
    jar: &CookieJar,
) -> Result<bool, GameError> {
    entitled_with(
        state,
        access.game_id,
        &access.access_mode,
        access.owner_user_id,
        viewer,
        jar,
    )
    .await
}

/// Same decision, for handlers that already selected `access_mode` and the
/// owner alongside their own storage lookup (the v86 routes).
pub(crate) async fn entitled_with(
    state: &AppState,
    game_id: i64,
    access_mode: &str,
    owner_user_id: i64,
    viewer: Option<&Claims>,
    jar: &CookieJar,
) -> Result<bool, GameError> {
    if access_mode != "invited" {
        return Ok(true);
    }
    if let Some(claims) = viewer {
        if crate::infrastructure::web::api::support::ownership::is_admin_or_mod(&claims.role) {
            return Ok(true);
        }
        let user_id = claims
            .user_id
            .parse::<i64>()
            .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
        if user_id == owner_user_id {
            return Ok(true);
        }
        let granted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_entitlements WHERE game_id = ? AND user_id = ?)",
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_one(&state.game_service.pool)
        .await?;
        if granted {
            return Ok(true);
        }
    }

    // Guests: the cookie is the whole case. A valid signature on the right
    // game, backed by a code that still exists and is not revoked.
    let Some(token) = jar.get(&invite_cookie_name(game_id)).map(|c| c.value()) else {
        return Ok(false);
    };
    let Some(claims) = decode_invite_token(state, token.to_string()).await else {
        return Ok(false);
    };
    if claims.game_id != game_id {
        return Ok(false);
    }
    let alive: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_invitations WHERE id = ? AND revoked_at IS NULL)",
    )
    .bind(claims.inv)
    .fetch_one(&state.game_service.pool)
    .await?;
    Ok(alive)
}

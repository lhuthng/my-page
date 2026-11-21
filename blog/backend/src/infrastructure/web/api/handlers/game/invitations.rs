// Invitation codes: one public redemption endpoint and the author's
// management surface. A redemption is atomic - the counter bump and the
// capacity check are one UPDATE, so two browsers racing a single-use code
// cannot both win - and hands the winning browser a signed cookie token
// (see access.rs) so the raw code never has to be stored client-side.
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;

use crate::domain::entities::secret::Claims;
use crate::domain::errors::game::GameError;
use crate::infrastructure::game_invites::{
    code_hint, generate_invitation_code, hash_invitation_code, normalize_invitation_code,
};
use crate::infrastructure::web::api::handlers::game::dto::{
    CreateInvitationRequest, CreateInvitationResponse, InvitationView, InvitationsResponse,
    RedeemInvitationRequest, RedeemInvitationResponse,
};
use crate::infrastructure::web::api::security::client_key;
use crate::infrastructure::web::api::support::ownership::require_owner;
use crate::infrastructure::web::server::AppState;

use super::access::{encode_invite_token, entitled, game_access_by_slug, invite_cookie};

/// Codes carry 80 bits of entropy; the cap exists so a leaked code cannot be
/// farmed for tokens from one address forever.
const REDEEM_IP_LIMIT: (usize, Duration) = (30, Duration::from_secs(600));

pub async fn redeem_invitation(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
    jar: CookieJar,
    headers: HeaderMap,
    Extension(opt_claims): Extension<Option<Claims>>,
    Json(body): Json<RedeemInvitationRequest>,
) -> Result<Response, GameError> {
    if let Err(crate::domain::errors::auth::AuthError::RateLimited { retry_after_secs }) =
        state.rate_limiter.check(
            &client_key("invite-redeem", &headers),
            REDEEM_IP_LIMIT.0,
            REDEEM_IP_LIMIT.1,
        )
    {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", retry_after_secs.to_string())],
            "Too many redemption attempts; try again later.".to_string(),
        )
            .into_response());
    }

    let Some(normalized) = normalize_invitation_code(&body.code) else {
        return Err(GameError::InvalidDemo(
            "That code does not look right. Codes look like XXXX-XXXX-XXXX-XXXX.".to_string(),
        ));
    };

    let pool = &state.game_service.pool;
    let access = game_access_by_slug(pool, &slug)
        .await?
        .ok_or(GameError::GameNotFound)?;
    if access.access_mode != "invited" {
        return Err(GameError::InvalidDemo(
            "This game is open and does not take invitation codes.".to_string(),
        ));
    }

    // Someone who can already play - through a grant or a cookie - redeems
    // nothing: re-entering a code they were sent again must not burn the
    // single use their other browser already spent.
    if entitled(&state, &access, opt_claims.as_ref(), &jar).await? {
        return Ok((
            jar,
            Json(RedeemInvitationResponse {
                entitled: true,
                access_mode: access.access_mode,
            }),
        )
            .into_response());
    }

    let row: Option<(i64, Option<String>)> = sqlx::query_as(
        "SELECT id, revoked_at FROM game_invitations WHERE game_id = ? AND code_hash = ?",
    )
    .bind(access.game_id)
    .bind(hash_invitation_code(&normalized))
    .fetch_optional(pool)
    .await?;
    let Some((invitation_id, revoked_at)) = row else {
        return Err(GameError::InvalidDemo(
            "Invalid invitation code.".to_string(),
        ));
    };
    if revoked_at.is_some() {
        return Err(GameError::InvalidDemo(
            "This code was revoked by the author.".to_string(),
        ));
    }

    // The whole redemption is this one statement: capacity checked and spent
    // in the same WHERE clause, under SQLite's write lock.
    let result = sqlx::query(
        "UPDATE game_invitations SET uses_count = uses_count + 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ? AND revoked_at IS NULL AND (max_uses = 0 OR uses_count < max_uses)",
    )
    .bind(invitation_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(GameError::InvalidDemo(
            "This code has already been used.".to_string(),
        ));
    }

    // A registered redeemer gets the account-bound grant (works on every
    // device); everyone additionally gets the browser cookie, which is what
    // the serving routes actually check.
    if let Some(claims) = &opt_claims {
        let user_id = claims
            .user_id
            .parse::<i64>()
            .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
        sqlx::query(
            "INSERT OR IGNORE INTO game_entitlements (game_id, user_id, invitation_id)
             VALUES (?, ?, ?)",
        )
        .bind(access.game_id)
        .bind(user_id)
        .bind(invitation_id)
        .execute(pool)
        .await?;
    }

    let token = encode_invite_token(&state, access.game_id, invitation_id).await?;
    Ok((
        jar.add(invite_cookie(access.game_id, token)),
        Json(RedeemInvitationResponse {
            entitled: true,
            access_mode: access.access_mode,
        }),
    )
        .into_response())
}

// --- the author's management surface (mod tier, owner-checked) --------------

#[derive(sqlx::FromRow)]
struct InvitationRow {
    id: i64,
    note: String,
    code_hint: String,
    max_uses: i64,
    uses_count: i64,
    revoked_at: Option<String>,
    created_at: String,
}

impl InvitationRow {
    fn into_view(self) -> InvitationView {
        InvitationView {
            id: self.id,
            note: self.note,
            hint: self.code_hint,
            max_uses: self.max_uses,
            uses_count: self.uses_count,
            revoked: self.revoked_at.is_some(),
            created_at: self.created_at,
        }
    }
}

pub async fn list_invitations(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<Json<InvitationsResponse>, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    require_owner(&state.game_service.pool, "games", game_id, user_id).await?;

    let rows: Vec<InvitationRow> = sqlx::query_as(
        "SELECT id, note, code_hint, max_uses, uses_count, revoked_at, created_at
         FROM game_invitations WHERE game_id = ? ORDER BY id DESC",
    )
    .bind(game_id)
    .fetch_all(&state.game_service.pool)
    .await?;
    Ok(Json(InvitationsResponse {
        invitations: rows.into_iter().map(InvitationRow::into_view).collect(),
    }))
}

pub async fn create_invitation(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    Json(body): Json<CreateInvitationRequest>,
) -> Result<(StatusCode, Json<CreateInvitationResponse>), GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    if body.max_uses < 0 {
        return Err(GameError::InvalidDemo(
            "A code's use count cannot be negative (0 means unlimited).".to_string(),
        ));
    }
    let note = body.note.unwrap_or_default().trim().to_string();
    require_owner(&state.game_service.pool, "games", game_id, user_id).await?;

    // 80 random bits make a hash collision a non-event; the loop is ceremony
    // against the UNIQUE constraint, not an expected path.
    for _ in 0..5 {
        let code = generate_invitation_code();
        let normalized = normalize_invitation_code(&code).ok_or_else(|| {
            GameError::InternalError("Generated code failed its own normalization".to_string())
        })?;
        let result = sqlx::query_as::<_, (i64, String)>(
            "INSERT INTO game_invitations (game_id, code_hash, code_hint, max_uses, uses_count, note, created_by)
             VALUES (?, ?, ?, ?, 0, ?, ?)
             RETURNING id, created_at",
        )
        .bind(game_id)
        .bind(hash_invitation_code(&normalized))
        .bind(code_hint(&normalized))
        .bind(body.max_uses)
        .bind(&note)
        .bind(user_id)
        .fetch_one(&state.game_service.pool)
        .await;
        let Ok((id, created_at)) = result else {
            // A collision (or any other insert failure) simply draws again.
            continue;
        };
        return Ok((
            StatusCode::CREATED,
            Json(CreateInvitationResponse {
                code,
                invitation: InvitationView {
                    id,
                    note,
                    hint: code_hint(&normalized),
                    max_uses: body.max_uses,
                    uses_count: 0,
                    revoked: false,
                    created_at,
                },
            }),
        ));
    }
    Err(GameError::InternalError(
        "Could not allocate a unique code".to_string(),
    ))
}

pub async fn revoke_invitation(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, invitation_id)): AxumPath<(i64, i64)>,
) -> Result<StatusCode, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    require_owner(&state.game_service.pool, "games", game_id, user_id).await?;

    let result = sqlx::query(
        "UPDATE game_invitations SET revoked_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND game_id = ? AND revoked_at IS NULL",
    )
    .bind(invitation_id)
    .bind(game_id)
    .execute(&state.game_service.pool)
    .await?;
    if result.rows_affected() == 0 {
        // An already-revoked code revokes to the same place; a code from
        // another game (or no code) does not exist.
        let exists: Option<i64> =
            sqlx::query_scalar("SELECT id FROM game_invitations WHERE id = ? AND game_id = ?")
                .bind(invitation_id)
                .bind(game_id)
                .fetch_optional(&state.game_service.pool)
                .await?;
        if exists.is_none() {
            return Err(GameError::GameNotFound);
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

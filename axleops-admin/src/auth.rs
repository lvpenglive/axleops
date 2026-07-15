use crate::users::{AuthContext, UserError};
use crate::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;

/// Authenticated caller: session user or legacy service token.
pub struct AuthUser(pub AuthContext);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Prefer session: Authorization: Bearer <token> or X-AxleOps-Session
        let bearer = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer ").map(str::trim));
        let session = parts
            .headers
            .get("X-AxleOps-Session")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        if let Some(token) = bearer.or(session) {
            return match state.users.resolve_session(token) {
                Ok(ctx) => Ok(AuthUser(ctx)),
                Err(UserError::Disabled) => Err((StatusCode::FORBIDDEN, "user disabled")),
                Err(_) => Err((StatusCode::UNAUTHORIZED, "invalid or expired session")),
            };
        }

        // Legacy / service automation token
        let provided = parts
            .headers
            .get("X-AxleOps-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !provided.is_empty() && provided == state.config.token {
            return Ok(AuthUser(AuthContext {
                user_id: None,
                username: "service-token".into(),
                role: "admin".into(),
                is_service: true,
                session_token: None,
            }));
        }

        Err((
            StatusCode::UNAUTHORIZED,
            "missing session or X-AxleOps-Token",
        ))
    }
}

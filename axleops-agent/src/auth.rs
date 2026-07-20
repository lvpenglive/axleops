use crate::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;

/// Validated token extracted from header `X-AxleOps-Token`.
pub struct AuthToken;

impl FromRequestParts<AppState> for AuthToken {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let provided = parts
            .headers
            .get("X-AxleOps-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let expected = state
            .token
            .read()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "token lock poisoned"))?;

        if provided.is_empty() || provided != expected.as_str() {
            return Err((
                StatusCode::UNAUTHORIZED,
                "invalid or missing X-AxleOps-Token",
            ));
        }
        Ok(AuthToken)
    }
}

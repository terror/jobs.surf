use super::*;

#[derive(Serialize, ToSchema)]
pub(crate) struct ErrorResponse {
  /// Human-readable error message.
  pub(crate) error: &'static str,
}

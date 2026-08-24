use super::*;

pub trait Adapter: Send + Sync {
  /// Normalizes a complete provider response.
  ///
  /// # Errors
  ///
  /// Returns an error when the response or any contained job does not match
  /// the provider's expected response shape.
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot>;
}

use super::*;

#[async_trait::async_trait]
pub trait Adapter: Send + Sync {
  /// Fetches and normalizes a complete provider snapshot.
  ///
  /// # Errors
  ///
  /// Returns an error if any request or normalization step fails.
  async fn fetch(&self) -> Result<JobSnapshot>;

  /// Normalizes a complete provider response.
  ///
  /// # Errors
  ///
  /// Returns an error when the response or any contained job does not match
  /// the provider's expected response shape.
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot>;
}

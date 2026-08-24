use super::*;

pub(crate) async fn get(
  client: &reqwest::Client,
  adapter: &'static str,
  identifier: &str,
  url: Url,
) -> Result<Vec<u8>> {
  let response = client
    .get(url)
    .timeout(std::time::Duration::from_mins(1))
    .send()
    .await
    .and_then(reqwest::Response::error_for_status)
    .map_err(|source| Error::Fetch {
      adapter,
      identifier: identifier.into(),
      source,
    })?;

  response
    .bytes()
    .await
    .map(|response| response.to_vec())
    .map_err(|source| Error::Fetch {
      adapter,
      identifier: identifier.into(),
      source,
    })
}

pub(crate) fn parse_url(
  adapter: &'static str,
  identifier: &str,
  url: String,
) -> Result<Url> {
  Url::parse(&url).map_err(|source| Error::ParseUrl {
    adapter,
    identifier: identifier.into(),
    source,
    url,
  })
}

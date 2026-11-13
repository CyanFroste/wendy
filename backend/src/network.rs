use anyhow::Result;
use anyhow::anyhow;
use reqwest::header::{
    AUTHORIZATION, COOKIE, HeaderMap, HeaderName, HeaderValue, LOCATION, PROXY_AUTHORIZATION,
};
use reqwest::redirect;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;
use url::Url;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpOptions {
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub ok: bool,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct HttpClient {
    client: Client,
    max_redirects: usize,
    max_response_bytes: usize,
}

impl HttpClient {
    pub fn new(max_redirects: usize, max_response_bytes: usize) -> Result<Self> {
        let client = Client::builder()
            .redirect(redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .user_agent("WendyWidgetFramework/1.0")
            .build()?;

        Ok(Self {
            client,
            max_redirects,
            max_response_bytes,
        })
    }

    pub async fn request(
        &self,
        url: &Url,
        options: Option<HttpOptions>,
        allowed_domains: &[impl AsRef<str>],
    ) -> Result<HttpResponse> {
        let options = options.unwrap_or_default();
        let mut method = options
            .method
            .as_deref()
            .unwrap_or("GET")
            .to_ascii_uppercase()
            .parse::<Method>()?;

        let mut header_map = HeaderMap::new();
        let mut body = options.body;

        if let Some(headers) = options.headers {
            for (k, v) in headers {
                let name = HeaderName::from_bytes(k.as_bytes())?;
                let val = HeaderValue::from_str(&v)?;

                header_map.insert(name, val);
            }
        }

        let timeout = Duration::from_millis(options.timeout_ms.unwrap_or(15_000).min(30_000));
        let mut current_url = validate_url(url, allowed_domains).await?;

        for _ in 0..=self.max_redirects {
            let mut req = self
                .client
                .request(method.clone(), current_url.clone())
                .headers(header_map.clone())
                .timeout(timeout);

            if let Some(ref payload) = body {
                req = req.body(payload.clone());
            }

            let res = req.send().await?;

            if res.status().is_redirection() {
                let location = res
                    .headers()
                    .get(LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow!("location header not found"))?;

                let next_url = current_url.join(location)?;
                let validated_next_url = validate_url(&next_url, allowed_domains).await?;

                if validated_next_url.host_str() != current_url.host_str() {
                    header_map.remove(AUTHORIZATION);
                    header_map.remove(COOKIE);
                    header_map.remove(PROXY_AUTHORIZATION);
                }

                current_url = validated_next_url;

                if matches!(res.status().as_u16(), 301..=303) {
                    method = Method::GET;
                    body = None;
                }

                continue;
            }

            let status = res.status().as_u16();
            let ok = res.status().is_success();
            let final_url = res.url().to_string();
            let mut headers = HashMap::new();

            for (k, v) in res.headers() {
                if let Ok(val) = v.to_str() {
                    headers.insert(k.to_string(), val.to_string());
                }
            }

            let bytes = res.bytes().await?;

            if bytes.len() > self.max_response_bytes {
                return Err(anyhow!(
                    "response exceeds maximum size. size: {} bytes (expected: {})",
                    bytes.len(),
                    self.max_response_bytes
                ));
            }

            return Ok(HttpResponse {
                status,
                ok,
                url: final_url,
                headers,
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }

        Err(anyhow!("too many redirects"))
    }
}

async fn validate_url(url: &Url, allowed_domains: &[impl AsRef<str>]) -> Result<Url> {
    let url = url.clone();

    if allowed_domains.is_empty() {
        return Err(anyhow!("no domains allowed in the manifest"));
    }

    if !matches!(url.scheme(), "http" | "https") {
        return Err(anyhow!("unsupported scheme: '{}'", url.scheme()));
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("host not found in URL"))?;
    let host_lower = host.to_ascii_lowercase();

    if host_lower == "localhost" || host_lower.ends_with(".local") {
        return Err(anyhow!("localhost is not allowed"));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(anyhow!("IP address '{host}' is blocked"));
        }
    }

    let domain_allowed = allowed_domains.iter().any(|allowed| {
        let d = allowed
            .as_ref()
            .trim()
            .to_ascii_lowercase()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();

        if d.is_empty() {
            return false;
        }

        if let Some(suffix) = d.strip_prefix("*.") {
            host_lower == suffix || host_lower.ends_with(&format!(".{suffix}"))
        } else {
            host_lower == d
        }
    });

    if !domain_allowed {
        return Err(anyhow!("domain '{host}' is not allowed in the manifest"));
    }

    let port = url.port_or_known_default().unwrap_or(443);

    for addr in tokio::net::lookup_host((host, port)).await? {
        if is_blocked_ip(addr.ip()) {
            return Err(anyhow!("resolved IP address '{host}' is blocked"));
        }
    }

    Ok(url)
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
        }
    }
}

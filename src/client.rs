use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

// const API_BASE_URL: &str = "https://localhost/api";
const API_BASE_URL: &str = "https://sf-dogfood/api";
const IGNORE_SSL_CERTIFICATE_VERIFICATION: bool = true;
const HTTP_BASIC_AUTH_USER: &str = "starfish";
const HTTP_BASIC_AUTH_PASSWORD: &str = "starfish";

pub struct Client {
    client: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(IGNORE_SSL_CERTIFICATE_VERIFICATION)
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                let auth = format!(
                    "Basic {}",
                    STANDARD.encode(format!(
                        "{}:{}",
                        HTTP_BASIC_AUTH_USER, HTTP_BASIC_AUTH_PASSWORD
                    ))
                );
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&auth).unwrap(),
                );
                headers
            })
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: API_BASE_URL.to_string(),
        }
    }

    pub async fn get_volumes(&self) -> Result<Value> {
        let url = format!("{}/volume/", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        Ok(response.json().await?)
    }

    pub async fn get_volume(&self, name: &str) -> Result<Option<Value>> {
        let url = format!("{}/volume/{}", self.base_url, name);
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            _ => Ok(Some(response.json().await?)),
        }
    }

    pub async fn get_scans(&self) -> Result<Value> {
        let url = format!("{}/scan/", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        Ok(response.json().await?)
    }

    pub async fn get_scan(&self, id: &str) -> Result<Option<Value>> {
        let url = format!("{}/scan/{}", self.base_url, id);
        let response = self.client
            .get(&url)
            .send()
            .await?;
        
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            eprintln!("Not authorized");
            std::process::exit(1);
        }

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            _ => Ok(Some(response.json().await?)),
        }
    }
} 
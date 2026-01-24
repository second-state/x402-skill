use crate::cli::Args;
use crate::error::X402Error;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Body, Method};
use std::fs;
use std::str::FromStr;

pub struct RequestConfig {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: Option<Body>,
    pub follow_redirects: bool,
}

impl RequestConfig {
    pub fn from_args(args: &Args) -> Result<Self, X402Error> {
        let method = Method::from_str(&args.method.to_uppercase())
            .map_err(|_| X402Error::General(format!("Invalid HTTP method: {}", args.method)))?;

        let headers = Self::parse_headers(&args.headers)?;
        let body = Self::parse_body(&args.data, &args.data_binary)?;

        Ok(RequestConfig {
            method,
            url: args.url.clone(),
            headers,
            body,
            follow_redirects: args.location,
        })
    }

    fn parse_headers(headers: &[String]) -> Result<HeaderMap, X402Error> {
        let mut map = HeaderMap::new();
        for header in headers {
            let parts: Vec<&str> = header.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(X402Error::General(format!("Invalid header format: {}", header)));
            }
            let name = HeaderName::from_str(parts[0].trim())
                .map_err(|_| X402Error::General(format!("Invalid header name: {}", parts[0])))?;
            let value = HeaderValue::from_str(parts[1].trim())
                .map_err(|_| X402Error::General(format!("Invalid header value: {}", parts[1])))?;
            map.insert(name, value);
        }
        Ok(map)
    }

    fn parse_body(data: &Option<String>, data_binary: &Option<String>) -> Result<Option<Body>, X402Error> {
        // data_binary takes precedence
        let data_str = data_binary.as_ref().or(data.as_ref());

        match data_str {
            Some(d) if d.starts_with('@') => {
                let path = &d[1..];
                let content = fs::read(path)
                    .map_err(|e| X402Error::General(format!("Failed to read file {}: {}", path, e)))?;
                Ok(Some(Body::from(content)))
            }
            Some(d) => Ok(Some(Body::from(d.clone()))),
            None => Ok(None),
        }
    }
}

use std::fmt;
use std::sync::Arc;

use reqwest::Url;

const LINEAR_UPLOADS: &str = "uploads.linear.app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageOrigin {
    LinearUpload,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImageFetchError {
    #[error("the image host returned HTTP {0}")]
    Status(u16),
    #[error("the image is larger than {0} bytes")]
    TooLarge(usize),
    #[error("the image could not be downloaded: {0}")]
    Transport(String),
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Located {
    url: Url,
    origin: ImageOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageUrl(Arc<Located>);

impl ImageUrl {
    pub fn parse(raw: &str) -> Option<Self> {
        let url = Url::parse(raw).ok()?;

        if url.scheme() != "https" {
            return None;
        }

        let linear = url.host_str() == Some(LINEAR_UPLOADS)
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none();
        let origin = if linear {
            ImageOrigin::LinearUpload
        } else {
            ImageOrigin::External
        };

        Some(ImageUrl(Arc::new(Located { url, origin })))
    }

    pub fn origin(&self) -> ImageOrigin {
        self.0.origin
    }

    pub fn url(&self) -> &Url {
        &self.0.url
    }

    pub fn as_str(&self) -> &str {
        self.0.url.as_str()
    }
}

impl fmt::Display for ImageUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_uploads_on_linear_carry_the_linear_origin() {
        let cases = [
            (
                "https://uploads.linear.app/a/b.png",
                Some(ImageOrigin::LinearUpload),
            ),
            (
                "https://attacker.example/i.png",
                Some(ImageOrigin::External),
            ),
            (
                "https://uploads.linear.app.attacker.example/i.png",
                Some(ImageOrigin::External),
            ),
            (
                "https://uploads.linear.app:8443/i.png",
                Some(ImageOrigin::External),
            ),
            (
                "https://user:pass@uploads.linear.app/i.png",
                Some(ImageOrigin::External),
            ),
            (
                "https://UPLOADS.LINEAR.APP/i.png",
                Some(ImageOrigin::LinearUpload),
            ),
            ("http://uploads.linear.app/i.png", None),
            ("file:///etc/passwd", None),
            ("data:image/png;base64,AAAA", None),
            ("not a url", None),
        ];

        for (raw, expected) in cases {
            assert_eq!(
                ImageUrl::parse(raw).map(|url| url.origin()),
                expected,
                "{raw}"
            );
        }
    }
}

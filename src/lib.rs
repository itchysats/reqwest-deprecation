use reqwest::Response;
use time::OffsetDateTime;

pub trait ResponseExt {
    fn deprecation(&self) -> Option<Deprecation>;
}

#[derive(Debug)]
pub struct Deprecation {
    /// The timestamp specified in the `Deprecation` header. If not set, means we encountered
    /// `Deprecation: true`.
    pub timestamp: Option<OffsetDateTime>,
    /// A link pointing to information about the deprecated resource.
    ///
    /// If present, we extracted it from a `Link` header with the relation `deprecation`.
    pub deprecation_link: Option<String>,
}

impl ResponseExt for Response {
    fn deprecation(&self) -> Option<Deprecation> {
        let value = self.headers().get(HEADER_NAME)?;

        let deprecation_link = self.headers().get_all("Link").iter().find_map(|link| {
            let value_as_str = link.to_str().ok()?;
            let link = parse_deprecation_link(value_as_str)?;

            Some(link.to_owned())
        });

        if value == "true" {
            return Some(Deprecation {
                timestamp: None,
                deprecation_link,
            });
        }

        let timestamp = OffsetDateTime::parse(
            value.to_str().ok()?,
            &time::format_description::well_known::Rfc2822,
        )
        .ok();

        Some(Deprecation {
            timestamp,
            deprecation_link,
        })
    }
}

const HEADER_NAME: &str = "Deprecation";

fn parse_deprecation_link(input: &str) -> Option<&str> {
    let mut parts = input.split(';').map(|p| p.trim());

    let url = parts.next()?.trim_start_matches('<').trim_end_matches('>');

    loop {
        let param = parts.next()?;
        let [key, value]: [&str; 2] = param.split('=').collect::<Vec<_>>().try_into().ok()?;

        if key == "rel" && value == r#""deprecation""# {
            return Some(url);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_deprecation_header() {
        let response: reqwest::Response = http::Response::builder()
            .header("Deprecation", "true")
            .status(200)
            .body(String::new())
            .unwrap()
            .into();

        let deprecation = response.deprecation().unwrap();

        assert_eq!(deprecation.timestamp, None);
        assert_eq!(deprecation.deprecation_link, None);
    }

    #[test]
    fn parse_deprecation_header_with_date() {
        let response: reqwest::Response = http::Response::builder()
            .header("Deprecation", "Thu, 01 Jan 1970 00:00:00 +0000")
            .status(200)
            .body(String::new())
            .unwrap()
            .into();

        let deprecation = response.deprecation().unwrap();

        assert_eq!(deprecation.timestamp, Some(OffsetDateTime::UNIX_EPOCH));
        assert_eq!(deprecation.deprecation_link, None);
    }

    #[test]
    fn parse_deprecation_header_with_invalid_date() {
        let response: reqwest::Response = http::Response::builder()
            .header("Deprecation", "2021-01-01T10:00:13Z") // ISO8601 is not valid HTTP header date
            .status(200)
            .body(String::new())
            .unwrap()
            .into();

        let deprecation = response.deprecation().unwrap();

        assert_eq!(deprecation.timestamp, None); // we still recognise deprecation but don't parse the date
        assert_eq!(deprecation.deprecation_link, None);
    }

    #[test]
    fn parse_deprecation_header_with_link() {
        let response: reqwest::Response = http::Response::builder()
            .header("Deprecation","true")
            .header("Link",r#"<https://developer.example.com/deprecation>; rel="deprecation"; type="text/html""#)
            .status(200)
            .body(String::new())
            .unwrap()
            .into();

        let deprecation = response.deprecation().unwrap();

        assert_eq!(deprecation.timestamp, None);
        assert_eq!(
            deprecation.deprecation_link,
            Some("https://developer.example.com/deprecation".to_owned())
        );
    }

    #[test]
    fn parse_deprecation_header_with_multiple_link() {
        let response: reqwest::Response = http::Response::builder()
            .header("Deprecation","true")
            .header("Link",r#"<https://example.com>; rel="alternate""#)
            .header("Link",r#"<https://developer.example.com/deprecation>; rel="deprecation"; type="text/html""#)
            .status(200)
            .body(String::new())
            .unwrap()
            .into();

        let deprecation = response.deprecation().unwrap();

        assert_eq!(deprecation.timestamp, None);
        assert_eq!(
            deprecation.deprecation_link,
            Some("https://developer.example.com/deprecation".to_owned())
        );
    }

    #[test]
    fn parse_link_header() {
        let link =
            r#"<https://developer.example.com/deprecation>; rel="deprecation"; type="text/html""#;

        let link = parse_deprecation_link(link).unwrap();

        assert_eq!(link, "https://developer.example.com/deprecation")
    }

    #[test]
    fn parse_link_header_different_order() {
        let link =
            r#"<https://developer.example.com/deprecation>;  type="text/html"; rel="deprecation";"#;

        let link = parse_deprecation_link(link).unwrap();

        assert_eq!(link, "https://developer.example.com/deprecation")
    }
}

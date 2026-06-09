use mime::Mime;
use serde::Serialize;

/// An in-memory HTTP request body with an optional MIME type.
///
/// Cheaply cloneable; all body data is in a `Vec<u8>` so there is no async
/// reading involved.
#[derive(Clone, Default)]
pub struct Body {
    bytes: Vec<u8>,
    mime: Option<Mime>,
}

impl Body {
    /// Consume the body and return its bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// The MIME type of the body, if one has been set.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn mime(&self) -> Option<&Mime> {
        self.mime.as_ref()
    }

    /// The number of bytes in the body.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `true` if the body is empty.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Create a body from a string with `text/plain; charset=utf-8` content type.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn from_string(s: String) -> Self {
        Self {
            bytes: s.into_bytes(),
            mime: Some(mime::TEXT_PLAIN_UTF_8),
        }
    }

    /// Serialize `value` to JSON bytes with `application/json` content type.
    ///
    /// # Errors
    /// Returns a `serde_json::Error` if serialization fails.
    pub fn from_json(value: &impl Serialize) -> Result<Self, serde_json::Error> {
        let bytes = serde_json::to_vec(value)?;
        Ok(Self {
            bytes,
            mime: Some(mime::APPLICATION_JSON),
        })
    }

    /// Serialize `value` as `application/x-www-form-urlencoded`.
    ///
    /// # Errors
    /// Returns a `serde_qs::Error` if serialization fails.
    pub fn from_form(value: &impl Serialize) -> Result<Self, serde_qs::Error> {
        let bytes = serde_qs::to_string(value)?.into_bytes();
        Ok(Self {
            bytes,
            mime: Some(mime::APPLICATION_WWW_FORM_URLENCODED),
        })
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl From<&str> for Body {
    fn from(s: &str) -> Self {
        Self::from_string(s.to_owned())
    }
}

impl From<Vec<u8>> for Body {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            mime: Some(mime::APPLICATION_OCTET_STREAM),
        }
    }
}

impl<'a> From<&'a [u8]> for Body {
    fn from(bytes: &'a [u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            mime: Some(mime::APPLICATION_OCTET_STREAM),
        }
    }
}

impl From<serde_json::Value> for Body {
    fn from(value: serde_json::Value) -> Self {
        // serde_json::Value always serialises without error.
        let bytes = serde_json::to_vec(&value).unwrap_or_default();
        Self {
            bytes,
            mime: Some(mime::APPLICATION_JSON),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_body_is_empty() {
        let body = Body::default();
        assert!(body.is_empty());
        assert_eq!(body.len(), 0);
        assert!(body.mime().is_none());
    }

    #[test]
    fn from_string_sets_text_plain_utf8() {
        let body = Body::from("hello".to_string());
        assert_eq!(body.into_bytes(), b"hello");
        // mime created from into() on String
        let body2 = Body::from_string("world".to_owned());
        assert_eq!(body2.mime().unwrap(), &mime::TEXT_PLAIN_UTF_8);
    }

    #[test]
    fn from_str_ref_sets_text_plain_utf8() {
        let body = Body::from("hello");
        assert_eq!(body.mime().unwrap(), &mime::TEXT_PLAIN_UTF_8);
    }

    #[test]
    fn from_bytes_sets_octet_stream() {
        let body = Body::from(vec![1u8, 2, 3]);
        assert_eq!(body.mime().unwrap(), &mime::APPLICATION_OCTET_STREAM);
        assert_eq!(body.into_bytes(), vec![1, 2, 3]);
    }

    #[test]
    fn from_byte_slice_sets_octet_stream() {
        let body = Body::from(&[4u8, 5, 6][..]);
        assert_eq!(body.mime().unwrap(), &mime::APPLICATION_OCTET_STREAM);
    }

    #[test]
    fn from_json_serialises_and_sets_application_json() {
        #[derive(serde::Serialize)]
        struct Foo {
            x: u32,
        }
        let body = Body::from_json(&Foo { x: 42 }).unwrap();
        assert_eq!(body.mime().unwrap(), &mime::APPLICATION_JSON);
        let parsed: serde_json::Value = serde_json::from_slice(&body.into_bytes()).unwrap();
        assert_eq!(parsed["x"], 42);
    }

    #[test]
    fn from_json_value_sets_application_json() {
        let val = serde_json::json!({"key": "value"});
        let body = Body::from(val);
        assert_eq!(body.mime().unwrap(), &mime::APPLICATION_JSON);
    }

    #[test]
    fn from_form_serialises_and_sets_form_urlencoded() {
        #[derive(serde::Serialize)]
        struct Form {
            name: String,
            age: u32,
        }
        let body = Body::from_form(&Form {
            name: "Alice".into(),
            age: 30,
        })
        .unwrap();
        assert_eq!(body.mime().unwrap(), &mime::APPLICATION_WWW_FORM_URLENCODED);
        let s = String::from_utf8(body.into_bytes()).unwrap();
        assert!(s.contains("name=Alice"));
        assert!(s.contains("age=30"));
    }
}

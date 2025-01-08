use http_types::Error;

use std::fmt;
use std::io;

/// An error occurred while decoding a response body to a string.
///
/// The error carries the encoding that was used to attempt to decode the body, and the raw byte
/// contents of the body. This can be used to treat un-decodable bodies specially or to implement a
/// fallback parsing strategy.
#[derive(Clone)]
pub struct DecodeError {
    /// The name of the encoding that was used to try to decode the input.
    pub encoding: String,
    /// The input data as bytes.
    pub data: Vec<u8>,
}

// Override debug output so you don't get each individual byte in `data` printed out separately,
// because it can be many megabytes large. The actual content is not that interesting anyways
// and can be accessed manually if it is required.
impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodeError")
            .field("encoding", &self.encoding)
            // Perhaps we can output the first N bytes of the response in the future
            .field("data", &format!("{} bytes", self.data.len()))
            .finish()
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not decode body as {}", &self.encoding)
    }
}

impl std::error::Error for DecodeError {}

/// Check if an encoding label refers to the UTF-8 encoding.
#[allow(dead_code)]
fn is_utf8_encoding(encoding_label: &str) -> bool {
    encoding_label.eq_ignore_ascii_case("utf-8")
        || encoding_label.eq_ignore_ascii_case("utf8")
        || encoding_label.eq_ignore_ascii_case("unicode-1-1-utf-8")
}

/// Decode a response body as utf-8.
///
/// # Errors
///
/// If the body cannot be decoded as utf-8, this function returns an `std::io::Error` of kind
/// `std::io::ErrorKind::InvalidData`, carrying a `DecodeError` struct.
#[cfg(not(feature = "encoding"))]
pub fn decode_body(bytes: Vec<u8>, content_encoding: Option<&str>) -> Result<String, Error> {
    if is_utf8_encoding(content_encoding.unwrap_or("utf-8")) {
        Ok(String::from_utf8(bytes).map_err(|err| {
            let err = DecodeError {
                encoding: "utf-8".to_string(),
                data: err.into_bytes(),
            };
            io::Error::new(io::ErrorKind::InvalidData, err)
        })?)
    } else {
        let err = DecodeError {
            encoding: "utf-8".to_string(),
            data: bytes,
        };
        Err(io::Error::new(io::ErrorKind::InvalidData, err).into())
    }
}

/// Decode a response body as the given content type.
///
/// If the input bytes are valid utf-8, this does not make a copy.
///
/// # Errors
///
/// If an unsupported encoding is requested, or the body does not conform to the requested
/// encoding, this function returns an `std::io::Error` of kind `std::io::ErrorKind::InvalidData`,
/// carrying a `DecodeError` struct.
#[cfg(all(feature = "encoding", not(target_arch = "wasm32")))]
pub fn decode_body(bytes: Vec<u8>, content_encoding: Option<&str>) -> Result<String, Error> {
    use encoding_rs::Encoding;
    use std::borrow::Cow;

    let content_encoding = content_encoding.unwrap_or("utf-8");
    if let Some(encoding) = Encoding::for_label(content_encoding.as_bytes()) {
        let (decoded, encoding_used, failed) = encoding.decode(&bytes);
        if failed {
            let err = DecodeError {
                encoding: encoding_used.name().into(),
                data: bytes,
            };
            Err(io::Error::new(io::ErrorKind::InvalidData, err).into())
        } else {
            Ok(match decoded {
                // If encoding_rs returned a `Cow::Borrowed`, the bytes are guaranteed to be valid
                // UTF-8, by virtue of being UTF-8 or being in the subset of ASCII that is the same
                // in UTF-8.
                Cow::Borrowed(_) => unsafe { String::from_utf8_unchecked(bytes) },
                Cow::Owned(string) => string,
            })
        }
    } else {
        let err = DecodeError {
            encoding: content_encoding.to_string(),
            data: bytes,
        };
        Err(io::Error::new(io::ErrorKind::InvalidData, err).into())
    }
}

/// Decode a response body as the given content type.
///
/// This always makes a copy. (It could be optimized to avoid the copy if the encoding is utf-8.)
///
/// # Errors
///
/// If an unsupported encoding is requested, or the body does not conform to the requested
/// encoding, this function returns an `std::io::Error` of kind `std::io::ErrorKind::InvalidData`,
/// carrying a `DecodeError` struct.
#[cfg(all(feature = "encoding", target_arch = "wasm32"))]
pub fn decode_body(mut bytes: Vec<u8>, content_encoding: Option<&str>) -> Result<String, Error> {
    use web_sys::TextDecoder;

    // Encoding names are always valid ASCII, so we can avoid including casing mapping tables
    let content_encoding = content_encoding.unwrap_or("utf-8").to_ascii_lowercase();
    if is_utf8_encoding(&content_encoding) {
        return String::from_utf8(bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err).into());
    }

    let decoder = TextDecoder::new_with_label(&content_encoding).unwrap();

    Ok(decoder.decode_with_u8_array(&mut bytes).map_err(|_| {
        let err = DecodeError {
            encoding: content_encoding.to_string(),
            data: bytes,
        };
        io::Error::new(io::ErrorKind::InvalidData, err)
    })?)
}

#[cfg(test)]
mod decode_tests {
    use super::decode_body;

    #[test]
    fn utf8() {
        let input = "Rød grød med fløde";
        assert_eq!(
            decode_body(input.as_bytes().to_vec(), Some("utf-8")).unwrap(),
            input,
            "Parses utf-8"
        );
    }

    #[test]
    fn default_utf8() {
        let input = "Rød grød med fløde";
        assert_eq!(
            decode_body(input.as_bytes().to_vec(), None).unwrap(),
            input,
            "Defaults to utf-8"
        );
    }

    #[test]
    fn euc_kr() {
        let input = vec![
            0xb3, 0xbb, 0x20, 0xc7, 0xb0, 0xc0, 0xb8, 0xb7, 0xce, 0x20, 0xb5, 0xb9, 0xbe, 0xc6,
            0xbf, 0xc0, 0xb6, 0xf3, 0x2c, 0x20, 0xb3, 0xbb, 0x20, 0xbe, 0xc8, 0xbf, 0xa1, 0xbc,
            0xad, 0x20, 0xc0, 0xe1, 0xb5, 0xe9, 0xb0, 0xc5, 0xb6, 0xf3,
        ];

        let result = decode_body(input, Some("euc-kr"));
        if cfg!(feature = "encoding") {
            assert_eq!(result.unwrap(), "내 품으로 돌아오라, 내 안에서 잠들거라");
        } else {
            assert!(result.is_err(), "Only utf-8 is supported");
        }
    }
}

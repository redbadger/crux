use crate::{http::Headers, response::new_headers};
use http_types::{
    StatusCode, Version,
    headers::{HeaderName, HeaderValue},
};

pub fn headers_ser(headers: &Headers) -> Result<Vec<(String, Vec<String>)>, &'static str> {
    Ok(headers
        .iter()
        .map(|(name, values)| {
            (
                name.to_string(),
                values
                    .iter()
                    .map(HeaderValue::to_string)
                    .collect::<Vec<_>>(),
            )
        })
        .collect())
}

pub fn headers_deser(strs: &Vec<(String, Vec<String>)>) -> Result<Headers, &'static str> {
    let mut headers = new_headers();
    for (name, values) in strs {
        let name = HeaderName::from_string(name.clone()).map_err(|_| "Invalid header name")?;
        for value in values {
            headers.append(&name, value);
        }
    }
    Ok(headers)
}

pub fn status_code_ser(code: &StatusCode) -> Result<u16, &'static str> {
    Ok((*code).into())
}
pub fn status_code_deser(v: &u16) -> Result<StatusCode, &'static str> {
    StatusCode::try_from(*v).map_err(|_| "Invalid Status Code")
}

pub fn version_ser(version: &Option<Version>) -> Result<Option<String>, &'static str> {
    Ok(version.as_ref().map(ToString::to_string))
}
pub fn version_deser(str: &Option<String>) -> Result<Option<Version>, &'static str> {
    str.as_ref()
        .map(|s| match s.as_str() {
            "HTTP/0.9" => Ok(Version::Http0_9),
            "HTTP/1.0" => Ok(Version::Http1_0),
            "HTTP/1.1" => Ok(Version::Http1_1),
            "HTTP/2" => Ok(Version::Http2_0),
            "HTTP/3" => Ok(Version::Http3_0),
            _ => Err("Invalid Http Version"),
        })
        .transpose()
}

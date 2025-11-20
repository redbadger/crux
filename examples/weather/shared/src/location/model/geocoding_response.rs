use facet::Facet;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::LazyLock};

use crate::location::Location;

pub const GEOCODING_URL: &str = "https://api.openweathermap.org/geo/1.0/direct";

#[derive(
    Facet, Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Clone,
)]
pub struct LocalNames {
    pub af: Option<String>,
    pub ar: Option<String>,
    pub ascii: Option<String>,
    pub az: Option<String>,
    pub bg: Option<String>,
    pub ca: Option<String>,
    pub da: Option<String>,
    pub de: Option<String>,
    pub el: Option<String>,
    pub en: Option<String>,
    pub eu: Option<String>,
    pub fa: Option<String>,
    pub feature_name: Option<String>,
    pub fi: Option<String>,
    pub fr: Option<String>,
    pub gl: Option<String>,
    pub he: Option<String>,
    pub hi: Option<String>,
    pub hr: Option<String>,
    pub hu: Option<String>,
    pub id: Option<String>,
    pub it: Option<String>,
    pub ja: Option<String>,
    pub la: Option<String>,
    pub lt: Option<String>,
    pub mk: Option<String>,
    pub nl: Option<String>,
    pub no: Option<String>,
    pub pl: Option<String>,
    pub pt: Option<String>,
    pub ro: Option<String>,
    pub ru: Option<String>,
    pub sk: Option<String>,
    pub sl: Option<String>,
    pub sr: Option<String>,
    pub th: Option<String>,
    pub tr: Option<String>,
    pub vi: Option<String>,
    pub zu: Option<String>,
}

impl fmt::Display for LocalNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LocalNames: (af: {}, ar: {}, ascii: {}, az: {}, bg: {}, ca: {}, da: {}, de: {}, el: {}, en: {}, eu: {}, fa: {}, feature_name: {}, fi: {}, fr: {}, gl: {}, he: {}, hi: {}, hr: {}, hu: {}, id: {}, it: {}, ja: {}, la: {}, lt: {}, mk: {}, nl: {}, no: {}, pl: {}, pt: {}, ro: {}, ru: {}, sk: {}, sl: {}, sr: {}, th: {}, tr: {}, vi: {}, zu: {})",
            display_option(self.af.as_ref()),
            display_option(self.ar.as_ref()),
            display_option(self.ascii.as_ref()),
            display_option(self.az.as_ref()),
            display_option(self.bg.as_ref()),
            display_option(self.ca.as_ref()),
            display_option(self.da.as_ref()),
            display_option(self.de.as_ref()),
            display_option(self.el.as_ref()),
            display_option(self.en.as_ref()),
            display_option(self.eu.as_ref()),
            display_option(self.fa.as_ref()),
            display_option(self.feature_name.as_ref()),
            display_option(self.fi.as_ref()),
            display_option(self.fr.as_ref()),
            display_option(self.gl.as_ref()),
            display_option(self.he.as_ref()),
            display_option(self.hi.as_ref()),
            display_option(self.hr.as_ref()),
            display_option(self.hu.as_ref()),
            display_option(self.id.as_ref()),
            display_option(self.it.as_ref()),
            display_option(self.ja.as_ref()),
            display_option(self.la.as_ref()),
            display_option(self.lt.as_ref()),
            display_option(self.mk.as_ref()),
            display_option(self.nl.as_ref()),
            display_option(self.no.as_ref()),
            display_option(self.pl.as_ref()),
            display_option(self.pt.as_ref()),
            display_option(self.ro.as_ref()),
            display_option(self.ru.as_ref()),
            display_option(self.sk.as_ref()),
            display_option(self.sl.as_ref()),
            display_option(self.sr.as_ref()),
            display_option(self.th.as_ref()),
            display_option(self.tr.as_ref()),
            display_option(self.vi.as_ref()),
            display_option(self.zu.as_ref()),
        )
    }
}

#[derive(Facet, Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone)]
pub struct GeocodingResponse {
    pub name: String,
    pub local_names: Option<LocalNames>,
    pub lat: f64,
    pub lon: f64,
    pub country: String,
    pub state: Option<String>,
}

impl GeocodingResponse {
    pub(crate) fn location(&self) -> Location {
        Location {
            lat: self.lat,
            lon: self.lon,
        }
    }
}

impl fmt::Display for GeocodingResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GeocodingResponse: (name: {}, local_names: {}, lat: {}, lon: {}, country: {}, state: {})",
            self.name,
            display_option(self.local_names.as_ref()),
            self.lat,
            self.lon,
            self.country,
            display_option(self.state.as_ref()),
        )
    }
}

#[allow(dead_code)] // TODO: why?
pub static SAMPLE_GEOCODING_RESPONSE: LazyLock<Vec<GeocodingResponse>> = LazyLock::new(|| {
    vec![GeocodingResponse {
        name: "Phoenix".to_string(),
        local_names: None,
        lat: 33.456_789,
        lon: -112.037_222,
        country: "US".to_string(),
        state: Some("Arizona".to_string()),
    }]
});

#[allow(dead_code)] // TODO: why?
pub static SAMPLE_GEOCODING_RESPONSE_JSON: LazyLock<String> =
    LazyLock::new(|| serde_json::to_string(&*SAMPLE_GEOCODING_RESPONSE).unwrap());

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone)]
#[allow(dead_code)] // TODO: why?
pub struct ZipCodeResponse {
    pub zip: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub country: String,
}

impl fmt::Display for ZipCodeResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ZipCodeResponse: (zip: {}, name: {}, lat: {}, lon: {}, country: {})",
            self.zip, self.name, self.lat, self.lon, self.country,
        )
    }
}

pub fn display_option<T: fmt::Display>(option_string: Option<&T>) -> String {
    match option_string {
        Some(string) => string.to_string(),
        None => "None".to_string(),
    }
}

#[derive(Serialize)]
pub struct GeocodingQueryString {
    pub q: String,
    pub limit: &'static str,
    pub appid: String,
}

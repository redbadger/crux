use serde::{Deserialize, Serialize};
use std::fmt;

use crate::responses::utils::display_option;

#[derive(Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash, Default, Clone)]
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
            display_option(&self.af),
            display_option(&self.ar),
            display_option(&self.ascii),
            display_option(&self.az),
            display_option(&self.bg),
            display_option(&self.ca),
            display_option(&self.da),
            display_option(&self.de),
            display_option(&self.el),
            display_option(&self.en),
            display_option(&self.eu),
            display_option(&self.fa),
            display_option(&self.feature_name),
            display_option(&self.fi),
            display_option(&self.fr),
            display_option(&self.gl),
            display_option(&self.he),
            display_option(&self.hi),
            display_option(&self.hr),
            display_option(&self.hu),
            display_option(&self.id),
            display_option(&self.it),
            display_option(&self.ja),
            display_option(&self.la),
            display_option(&self.lt),
            display_option(&self.mk),
            display_option(&self.nl),
            display_option(&self.no),
            display_option(&self.pl),
            display_option(&self.pt),
            display_option(&self.ro),
            display_option(&self.ru),
            display_option(&self.sk),
            display_option(&self.sl),
            display_option(&self.sr),
            display_option(&self.th),
            display_option(&self.tr),
            display_option(&self.vi),
            display_option(&self.zu),
        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone)]
pub struct GeocodingResponse {
    pub name: String,
    pub local_names: Option<LocalNames>,
    pub lat: f64,
    pub lon: f64,
    pub country: String,
    pub state: Option<String>,
}

impl fmt::Display for GeocodingResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GeocodingResponse: (name: {}, local_names: {}, lat: {}, lon: {}, country: {}, state: {})",
            self.name,
            display_option(&self.local_names),
            self.lat,
            self.lon,
            self.country,
            display_option(&self.state),
        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Clone)]
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

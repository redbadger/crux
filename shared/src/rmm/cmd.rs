use super::{
    capability::Envelope,
    http::Http,
    key_value::KeyValueRead,
    key_value::{KeyValue, KeyValueWrite},
    platform::Platform,
    time::Time,
};

// TODO consider whether these fields should be public
pub struct Cmd<Msg> {
    pub http: Http<Msg>,
    pub time: Time<Msg>,
    pub key_value_read: KeyValueRead<Msg>,
    pub key_value_write: KeyValueWrite<Msg>,
    pub platform: Platform<Msg>,
}

impl<Msg> Default for Cmd<Msg> {
    fn default() -> Self {
        Self {
            http: Http::default(),
            time: Time::default(),
            key_value_read: KeyValueRead::default(),
            key_value_write: KeyValueWrite::default(),
            platform: Platform::default(),
        }
    }
}

// These type aliases are not ideal but needed for FFI,
// due to current lack of generics support in uniffi
pub type StringEnvelope = Envelope<String>;
pub type BytesEnvelope = Envelope<Vec<u8>>;
pub type BoolEnvelope = Envelope<bool>;
pub type OptionalBoolEnvelope = Envelope<Option<bool>>;
pub type OptionalBytesEnvelope = Envelope<Option<Vec<u8>>>;
pub type KeyValueEnvelope = Envelope<KeyValue>;

pub enum Request {
    Http { data: StringEnvelope },
    Time { data: OptionalBoolEnvelope },
    Platform { data: OptionalBoolEnvelope },
    KVRead { data: StringEnvelope },
    KVWrite { data: KeyValueEnvelope },
    Render,
}

pub enum Response {
    Http { data: BytesEnvelope },
    Time { data: StringEnvelope },
    Platform { data: StringEnvelope },
    KVRead { data: OptionalBytesEnvelope },
    KVWrite { data: BoolEnvelope },
}

//! TODO mod docs

use crate::{Command, RequestBody, ResponseBody};

/// TODO docs
pub fn read<F, Message>(key: String, msg: F) -> Command<Message>
where
    F: FnOnce(Option<Vec<u8>>) -> Message + Sync + Send + 'static,
{
    let body = RequestBody::KVRead(key);

    Command {
        body: body.clone(),
        msg_constructor: Some(Box::new(move |rb: ResponseBody| {
            if let ResponseBody::KVRead(data) = rb {
                return msg(data);
            }

            panic!(
                "Attempt to continue KVRead request with different response {:?}",
                body
            );
        })),
    }
}

/// TODO docs
pub fn write<F, Message>(key: String, value: Vec<u8>, msg: F) -> Command<Message>
where
    F: FnOnce(bool) -> Message + Sync + Send + 'static,
{
    let body = RequestBody::KVWrite(key, value);

    Command {
        body: body.clone(),
        msg_constructor: Some(Box::new(move |rb| {
            if let ResponseBody::KVWrite(data) = rb {
                return msg(data);
            }

            panic!(
                "Attempt to continue KVWrite request with different response {:?}",
                body
            );
        })),
    }
}

//! TODO mod docs

use crate::{Command, RequestBody, ResponseBody};

/// TODO docs
pub fn get<F, Message>(url: String, msg: F) -> Command<Message>
where
    F: FnOnce(Vec<u8>) -> Message + Sync + Send + 'static,
{
    let body = RequestBody::Http(url);

    Command {
        body: body.clone(),
        msg_constructor: Some(Box::new(move |rb| {
            if let ResponseBody::Http(data) = rb {
                return msg(data);
            }

            panic!(
                "Attempt to continue HTTP request with different response {:?}",
                body
            );
        })),
    }
}

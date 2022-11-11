use crate::{Command, RequestBody, ResponseBody};

pub fn get<F, Message>(msg: F) -> Command<Message>
where
    F: FnOnce(String) -> Message + Sync + Send + 'static,
{
    let body = RequestBody::Time;

    Command {
        body: body.clone(),
        msg_constructor: Some(Box::new(move |rb| {
            if let ResponseBody::Time(data) = rb {
                return msg(data);
            }

            panic!(
                "Attempt to continue Time request with different response {:?}",
                body
            );
        })),
    }
}

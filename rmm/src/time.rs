use crate::{Command, RequestBody, ResponseBody};

pub fn get<F, Message>(
    msg: F,
) -> Command<impl FnOnce(ResponseBody) -> Message + Sync + Send + 'static, Message>
where
    F: FnOnce(String) -> Message + Sync + Send + 'static,
{
    let body = RequestBody::Platform;

    Command {
        body: body.clone(),
        msg_constructor: move |rb| {
            if let ResponseBody::Time(data) = rb {
                return msg(data);
            }

            panic!(
                "Attempt to continue Time request with different response {:?}",
                body
            );
        },
    }
}

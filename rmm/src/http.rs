use crate::{Command, RequestBody, ResponseBody};

pub fn get<F, Msg>(
    url: String,
    msg: F,
) -> Command<impl FnOnce(ResponseBody) -> Msg + Sync + Send + 'static, Msg>
where
    F: FnOnce(Vec<u8>) -> Msg + Sync + Send + 'static,
{
    let body = RequestBody::Http(url);

    Command {
        body: body.clone(),
        msg_constructor: move |rb| {
            if let ResponseBody::Http(data) = rb {
                return msg(data);
            }

            panic!(
                "Attempt to continue HTTP request with different response {:?}",
                body
            );
        },
    }
}

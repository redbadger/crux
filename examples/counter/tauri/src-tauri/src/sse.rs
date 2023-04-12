use crate::error::Error;
use async_std::io::ReadExt;
use futures::stream;

pub async fn sse(url: String) -> Result<impl futures::stream::TryStream<Ok = Vec<u8>>, Error> {
    let url = surf::Url::parse(&url)?;
    let mut response = surf::get(&url).await?;
    let status = response.status().into();

    let body = if let 200..=299 = status {
        response.take_body()
    } else {
        return Err(Error::HttpResponse(status, "SSE error".to_string()));
    };

    let body = body.into_reader();

    Ok(Box::pin(stream::try_unfold(body, move |mut body| async {
        let mut buf = [0; 1024];

        match body.read(&mut buf).await {
            Ok(n) if n == 0 => Ok(None),
            Ok(n) => Ok(Some((buf[0..n].to_vec(), body))),
            Err(e) => Err(Error::HttpDecode(format!(
                "failed to read from http response; err = {:?}",
                e
            ))),
        }
    })))
}

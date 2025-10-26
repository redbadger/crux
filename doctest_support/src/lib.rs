#![deny(clippy::pedantic)]
//! This is support code for doc tests

pub mod basic_delay;
pub mod delay;

pub mod command {
    use crux_core::{Request, capability::Operation};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    pub enum AnOperation {
        One(u8),
        Two(u8),
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub enum AnOperationOutput {
        One(u8),
        Two(u8),
    }

    impl Operation for AnOperation {
        type Output = AnOperationOutput;
    }

    pub enum Effect {
        AnEffect(Request<AnOperation>),
        Http(Request<crux_http::protocol::HttpRequest>),
        Render(Request<crux_core::render::RenderOperation>),
    }

    impl From<Request<AnOperation>> for Effect {
        fn from(request: Request<AnOperation>) -> Self {
            Self::AnEffect(request)
        }
    }

    impl From<Request<crux_http::protocol::HttpRequest>> for Effect {
        fn from(request: Request<crux_http::protocol::HttpRequest>) -> Self {
            Self::Http(request)
        }
    }

    impl From<Request<crux_core::render::RenderOperation>> for Effect {
        fn from(request: Request<crux_core::render::RenderOperation>) -> Self {
            Self::Render(request)
        }
    }

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    pub struct Post {
        pub url: String,
        pub title: String,
        pub body: String,
    }

    #[derive(Debug, PartialEq)]
    pub enum Event {
        Start,
        Completed(AnOperationOutput),
        Aborted,
        GotPost(Result<crux_http::Response<Post>, crux_http::HttpError>),
    }

    #[cfg(test)]
    mod tests {
        use crux_http::{
            command::Http,
            protocol::{HttpRequest, HttpResponse, HttpResult},
            testing::ResponseBuilder,
        };

        use crate::command::{Effect, Event, Post};

        #[test]
        fn http_post() {
            const API_URL: &str = "https://example.com/api/posts";

            // Create a command to post a new Post to API_URL
            // and then dispatch an event with the result
            let mut cmd = Http::post(API_URL)
                .body(serde_json::json!({"title":"New Post", "body":"Hello!"}))
                .expect_json()
                .build()
                .then_send(Event::GotPost);

            // Check the effect is an HTTP request ...
            let effect = cmd.effects().next().unwrap();
            let Effect::Http(mut request) = effect else {
                panic!("Expected a HTTP effect")
            };

            // ... and the request is a POST to API_URL
            assert_eq!(
                &request.operation,
                &HttpRequest::post(API_URL)
                    .header("content-type", "application/json")
                    .body(r#"{"body":"Hello!","title":"New Post"}"#)
                    .build()
            );

            // Resolve the request with a successful response
            let body = Post {
                url: API_URL.to_string(),
                title: "New Post".to_string(),
                body: "Hello!".to_string(),
            };
            request
                .resolve(HttpResult::Ok(HttpResponse::ok().json(&body).build()))
                .expect("Resolve should succeed");

            // Check the event is a GotPost event with the successful response
            let actual = cmd.events().next().unwrap();
            let expected = Event::GotPost(Ok(ResponseBuilder::ok().body(body).build()));
            assert_eq!(actual, expected);

            assert!(cmd.is_done());
        }
    }
}

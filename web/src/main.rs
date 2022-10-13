use shared::{add_for_platform, Platform, PlatformError};
use web_sys::window;
use woothee::parser::Parser;
use yew::prelude::*;

#[function_component(HelloWorld)]
fn hello_world() -> Html {
    struct WebPlatform;
    impl Platform for WebPlatform {
        fn get(&self) -> Result<String, PlatformError> {
            let navigator = window().unwrap().navigator();
            let agent = navigator.user_agent().unwrap_or_default();
            let parser = Parser::new();
            Ok(parser.parse(&agent).unwrap_or_default().name.to_string())
        }
    }
    let result = add_for_platform(1, 2, Box::new(WebPlatform {})).unwrap_or_default();
    html! {
        <section class="section title has-text-centered">
            {result}
        </section>
    }
}

fn main() {
    yew::start_app::<HelloWorld>();
}

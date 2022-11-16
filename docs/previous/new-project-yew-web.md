# Yew Web App

[Table of Contents](./new-project.md)

1. Install [`trunk`](https://github.com/thedodd/trunk)
1. Create a new Rust binary project

   ```sh
   cargo new web-yew
   ```

1. Edit [`./web/Cargo.toml`](../web/Cargo.toml) to add the new app to the Cargo workspace ...

   ```toml
   [workspace]
   members = ["shared", "web-yew"]
   ```

1. Add [`yew`](https://yew.rs/) and the shared library as dependencies in [`./web/Cargo.toml`](./web/Cargo.toml)

   ```toml
   [dependencies]
   yew = "0.19.3"
   shared = { path = "../shared" }
   ```

1. Add [`./web/index.html`](../web/index.html) ...

   ```html
   <!DOCTYPE html>
   <html>
     <head>
       <meta charset="utf-8" />
       <title>Yew App</title>
     </head>
   </html>
   ```

1. Edit [`./web/src/main.rs`](../web/src/main.rs), for example ...

   ```rust
   use shared::add;
   use yew::prelude::*;

   struct WebPlatform;
   impl Platform for WebPlatform {
      fn get(&self) -> Result<String, PlatformError> {
            let navigator = window().unwrap().navigator();
            let agent = navigator.user_agent().unwrap_or_default();
            let parser = Parser::new();
            Ok(parser.parse(&agent).unwrap_or_default().name.to_string())
      }
   }

   #[function_component(HelloWorld)]
   fn hello_world() -> Html {
      html! {
         <p>{"1 + 2 = "}{add_for_platform(1, 2, Box::new(WebPlatform {})).unwrap_or_default()}</p>
      }
   }

   fn main() {
      yew::start_app::<HelloWorld>();
   }
   ```

1. Build and serve the web page

   ```sh
   cd ./web-yew
   trunk serve
   ```

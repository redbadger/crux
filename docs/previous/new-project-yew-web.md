# Yew Web App

[Table of Contents](./new-project.md)

1. Install [`trunk`](https://github.com/thedodd/trunk)
1. Create a new Rust binary project

   ```sh
   cargo new web-yew
   ```

1. Edit `Cargo.toml` to add the new app to the Cargo workspace ...

   ```toml
   [workspace]
   members = ["shared", "web-yew"]
   ```

1. Add [`yew`](https://yew.rs/) and the shared library as dependencies in `Cargo.toml`

   ```toml
   [dependencies]
   yew = "0.19.3"
   shared = { path = "../shared" }
   ```

1. Add an `index.html` ...

   ```html
   <!DOCTYPE html>
   <html>
     <head>
       <meta charset="utf-8" />
       <title>Yew App</title>
     </head>
   </html>
   ```

1. Build and serve the web page

   ```sh
   cd ./web-yew
   trunk serve
   ```

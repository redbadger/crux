# counter_flutter

Crux **counter** shell using [flutter_rust_bridge](https://cjycode.com/flutter_rust_bridge/).

From this directory:

- `just doctor` — toolchain checks  
- `just generate` — regenerate `rust/src/frb_generated.rs` and `lib/src/rust/`  
- `just rust-build` — build the `counter_flutter` cdylib (host target)  
- `just run` — `flutter run` on the current OS desktop target (`macos` / `linux` / `windows`)

On **macOS**, the Runner target runs `macos/scripts/build_counter_flutter.sh` during the Xcode build so `counter_flutter.framework` is copied into the app bundle (same idea as the Weather example). Other platforms still need their own FRB/native wiring if `dlopen` fails.

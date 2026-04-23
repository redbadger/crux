// ANCHOR: main
fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(web_leptos::App);
}
// ANCHOR_END: main

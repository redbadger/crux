use shared::add;
use yew::prelude::*;

#[function_component(HelloWorld)]
fn hello_world() -> Html {
    html! {
        <section class="section title has-text-centered">
            {"1 + 2 = "}{add(1, 2)}
        </section>
    }
}

fn main() {
    yew::start_app::<HelloWorld>();
}

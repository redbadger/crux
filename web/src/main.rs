use shared::add;
use yew::prelude::*;

#[function_component(HelloWorld)]
fn hello_world() -> Html {
    html! {
        <p>{"1 + 2 = "}{add(1, 2)}</p>
    }
}

fn main() {
    yew::start_app::<HelloWorld>();
}

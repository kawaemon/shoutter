mod pages;

use yew::prelude::*;
use yew_router::{BrowserRouter, Switch};

use crate::pages::{switch, Route};

#[function_component]
fn App() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

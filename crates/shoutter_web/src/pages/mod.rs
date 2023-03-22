pub mod home;
pub mod login;

use home::Home;
use login::Login;
use yew::{html, Html};
use yew_router::Routable;

#[derive(Debug, Clone, Copy, PartialEq, Routable)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
}

pub fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html!(<Home />),
        Route::Login => html!(<Login />),
    }
}

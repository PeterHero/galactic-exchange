mod transport;

use std::sync::{Arc, Mutex};

use rouille::{Request, Response};

use transport::auth::{post_login_response, post_register_response};

use galactic_exchange::GalacticExchange;

#[macro_use]
extern crate rouille;

fn get_health(_: &Request) -> Response {
    Response::text("").with_status_code(200)
}

fn main() {
    println!("Hello, galaxy!!");

    println!("Now listening on 0.0.0.0:8080");

    let exchange = Arc::new(Mutex::new(GalacticExchange::new()));

    rouille::start_server("0.0.0.0:8080", {
        let exchange = Arc::clone(&exchange);
        move |request| {
            router!(request,
                (GET) (/health) => { get_health(request) },
                (POST) (/register) => { post_register_response(request, &mut exchange.lock().unwrap()) },
                (POST) (/login) => { post_login_response(request, &mut exchange.lock().unwrap()) },
                _ => Response::empty_404()
            )
        }
    });
}

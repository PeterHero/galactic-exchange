#[macro_use]
extern crate rouille;

fn main() {
    println!("Hello, galaxy!!");
    println!("Now listening on 0.0.0.0:8080");

    rouille::start_server("0.0.0.0:8080", move |request| {
        router!(request,
            (GET) (/health) => {
                rouille::Response::text("").with_status_code(200)
            },
            _ => rouille::Response::empty_404()
        )
    });
}

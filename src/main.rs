mod proxy;
mod utils;

use proxy::Proxy;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let proxy = Proxy::new();

    proxy.run();
}

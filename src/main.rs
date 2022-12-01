mod proxy;
mod utils;

use proxy::Proxy;

fn main() {
    let proxy = Proxy::new();

    proxy.run();
}

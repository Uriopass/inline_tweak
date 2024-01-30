use inline_tweak::*;
use std::time::Duration;

fn main() {
    loop {
        println!("{} {}", tweak!(2.5), tweak!(35));
        std::thread::sleep(Duration::from_millis(500))
    }
}

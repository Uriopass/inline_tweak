use inline_tweak::*;
use std::time::Duration;

fn main() {
    loop {
        println!("{}", tweak!(5.5)); // Try changing the value while the application is running
        std::thread::sleep(Duration::from_millis(10))
    }
}

use inline_tweak::{release_tweak, watch};

fn main() {
    loop {
        // Try changing the value while the application is running (even in release mode)
        println!("{}", release_tweak!(1.5));
        watch!();
    }
}

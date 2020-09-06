use inline_tweak::tweak;
use std::time::Duration;

fn counter() -> i32 {
    static mut N: i32 = 0;
    unsafe {
        N += 1;
        N
    }
}

fn main() {
    loop {
        // Try removing or changing the value while the application is running
        println!("{}", tweak!(100; counter()));
        std::thread::sleep(Duration::from_millis(500));
    }
}

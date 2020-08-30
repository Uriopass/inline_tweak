use inline_tweak::{tweak, watch};

fn main() {
    loop {
        println!("{}", tweak!(4.5)); // Try changing the text while the application is running
        watch!(); // The thread will sleep here until anything in the file changes
    }
}

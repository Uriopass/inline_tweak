use inline_tweak::{tweak, watch};

fn main() {
    loop {
        println!("{}", tweak!("Lorem ipsum")); // Try changing the text while the application is running
        println!("{}", tweak!("габарит не выбран")); // Supports non ascii text
        watch!(); // The thread will sleep here until anything in the file changes
    }
}

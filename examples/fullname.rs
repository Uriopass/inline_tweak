fn main() {
    loop {
        println!("{}", inline_tweak::tweak!(11.5)); // Try changing the value while the application is running
        inline_tweak::watch!();
    }
}

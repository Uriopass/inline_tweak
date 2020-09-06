use inline_tweak::tweak;

fn main() {
    loop {
        let v =
tweak!(15.5);
        println!("{}", v); // Try changing the value while the application is running
        inline_tweak::watch!();
    }
}

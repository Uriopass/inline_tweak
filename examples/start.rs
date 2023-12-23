use inline_tweak::tweak;

fn main() {
    loop {
        let v = tweak!(16.5);
        let v2 = inline_tweak::tweak!(16.5);
        println!("{} {}", v, v2); // Try changing the value while the application is running
        inline_tweak::watch!();
    }
}

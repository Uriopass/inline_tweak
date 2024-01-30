#[inline_tweak::tweak_fn]
fn main() {
    loop {
        let char = 'c';
        let bool = true;
        let v = 1.0 + 5.0;
        println!("{} {} {}", v, char, bool);
        inline_tweak::watch!();
    }
}

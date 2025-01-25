#[inline_tweak::tweak_fn]
fn main() {
    loop {
        let char = 'c';
        let bool = true;
        let v = 1.0 + 7.0;
        let s = "hello";
        println!("{} {} {} {}", v, char, bool, s);
        inline_tweak::watch!();
    }
}

#[inline_tweak::release_tweak_fn]
fn main() {
    loop {
        let v = 3;
        println!("{}", v);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

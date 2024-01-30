#[inline_tweak::tweak_fn]
fn main() {
    loop {
        const C: i32 = 3;
        static V: i32 = 3;
        let v: [f32; 1] = [1.0];
        let test: &str = "hmm";
        let ok: f32 = 4.0;
        println!("{}{}", test, ok);
        inline_tweak::watch!();
    }
}

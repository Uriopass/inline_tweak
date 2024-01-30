#[inline_tweak::tweak_fn]
fn main() {
    loop {
        const C: i32 = 3;
        static V: i32 = 3;
        let v: [f32; 1] = [1.0];
        let ok: f32 = 1.0;
        println!("{}", ok);
        inline_tweak::watch!();
    }
}

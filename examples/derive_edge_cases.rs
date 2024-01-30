#[inline_tweak::tweak_fn]
fn main() {
    loop {
        const C: i32 = 3;
        static V: i32 = 3;
        let v: [f32; 1] = [1.0];
        let test: &str = "hmm";
        let ok: f32 = 5.0f32;
        let underscores: i32 = 1_000;
        let radix: i32 = 0xFF;

        let s = "multi
        lines
        strings!";
        println!("{} {} {} {}", s, ok, underscores, radix);
        inline_tweak::watch!();
    }
}

use inline_tweak::{tweak, watch};

fn a() -> f32 {
    tweak!(1.5)
}

fn b() -> f32 {
    tweak!(2.5)
}

fn main() {
    loop {
        let b = b();
        let a = a();

        println!("a:{} b:{}", a, b);
        watch!(); // The thread will sleep here until anything in the file changes
    }
}

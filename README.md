# inline_tweak

![](https://i.imgur.com/DZrg910.gif)

[![Crates.io](https://img.shields.io/crates/v/inline_tweak.svg)](https://crates.io/crates/inline_tweak)

Tweak any literal directly from your code, changes to the source appear while running the program.  
It works by parsing the file when a change occurs.  

The library is minimal with **0** dependencies.  
In release mode, the tweaking code is disabled and compiled away.

The `derive` feature exposes a proc macro to turn all literals from a function body into tweakable values.

**inline_tweak** is based on [this blogpost](http://blog.tuxedolabs.com/2018/03/13/hot-reloading-hardcoded-parameters.html)
by tuxedo labs.

## Usage

```rust
use inline_tweak::*;

fn main() {
    loop {
        println!("{}", tweak!(3.14)); // Try changing the value while the application is running
    }
}
```

## Extra features

### derive

The `derive` feature allows to tweak any number/bool/char literal in a function.
It avoids cluttering the code with `inline_tweak::tweak!` calls.

```rust
#[inline_tweak::tweak_fn]
fn main() {
    loop {
       let v = 1.0; // Try changing this value!
       println!("{}", v);
       std::thread::sleep(Duration::from_millis(200)); // or even this value :)
    }
}
```

Note that it requires `syn`/`quote`/`proc_macro2` dependencies which makes the crate slower to compile.  
Contrary to `tweak!`, it does not allow tweaking literals in macro calls (like `println!`), as it cannot reliably replace literals by a function call since macros can have custom syntax.

#### watch!

`inline_tweak` provides a `watch!()` macro that sleeps until the file is modified, akin to a breakpoint:
```rust
use inline_tweak::*;

fn main() {
    loop {
        println!("{}", tweak!("hello world"));
        watch!(); // The thread will sleep here until anything in the file changes
    }
}
```

#### Expressions

`inline_tweak` allows to tweak expressions by providing a value later.
For example:
```rust
tweak!(rng.gen_range(0.0, 1.0))
``` 

can then be replaced by a constant value by modifying the file (even while the application is running) to
```rust
tweak!(5.0; rng.gen_range(0.0, 1.0)) // will always return 5.0
```

[See the "expression" example in action](https://i.imgur.com/pSvLNlI.mp4)

Note that this works only for expressions that return a tweakable type. (number/boolean literals)

#### release_tweak!

The `release_tweak!` macro acts exactly like `tweak!` except that it also works in release mode.  
It is accessible behind the feature flag `"release_tweak"` which is not enabled by default.  

## Installation

Simply add this line to your Cargo.toml

```toml
inline_tweak = "1.2"
```

# inline_tweak

![](https://i.imgur.com/DZrg910.gif)

[![Crates.io](https://img.shields.io/crates/v/inline_tweak.svg)](https://crates.io/crates/inline_tweak)

**inline_tweak** is based on [this blogpost](http://blog.tuxedolabs.com/2018/03/13/hot-reloading-hardcoded-parameters.html)
by tuxedo labs.  

Tweak any literal directly from your code, changes to the source appear while running the program.  
It works by parsing the file when a change occurs.  

The library is minimal, only requiring the `lazy_static` dependency to hold modified values.  
In release mode, the tweaking code is disabled and compiled away.

When targeting `Web/wasm32` all macros will compile but run a no-op, so reloading is not supported for this platform at the moment. 

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
inline_tweak = "1.0.10"
```

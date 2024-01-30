# Changelog

All notable changes to this project will be documented in this file.

## [1.0.12]

 - Introduce `derive` feature to allow `#[tweak_fn]` on functions.
 - All calls to tweak are now almost always at most one hashmap lookup.
 - Cache `stat` syscalls.
 - Only read file from disk once per file change instead of per tweak in that file.

These optimizations heavily reduce the overhead of using tweak!  
This allows the proc-macro adding tweaks to all literals of not being too heavy.

## [1.0.11]

 - Fix release_tweak! feature was broken

## [1.0.10]

 - Add explicit `wasm32` compile conditions to avoid runtime panics

## [1.0.9]

 - Performance improvement when using lots of `tweak!`s on unchanged files

## [1.0.8]

 - Support non ascii text literals
 - Allow implementing custom Tweakable types

## [1.0.7]

 - Add release_tweak! macro and feature
 - Support text literals

## [1.0.6]

 - Support expressions by providing a constant value if desired

## [1.0.5]

 - Allow full path to be used, `inline_tweak::tweak!` for example

## [1.0.4]

 - Fix  multiple `tweak!` not working if not called in order

## [1.0.3]

 - Allow `tweak!`s to move to a different line at runtime while still being correctly parsed.

## [1.0.2]

 - Add the `watch!` macro
 
## [1.0.1]

 - Allow multiple `tweak!` on the same line

## [1.0.0]
 - Add the tweak! macro to change number/boolean literals from source at runtime.

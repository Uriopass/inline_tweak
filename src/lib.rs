//! Tweak any literal directly from your code, changes to the source appear while running the program.
//! It works by parsing the file when a change occurs.
//!
//! The library is minimal with 0 dependencies.
//! In release mode, the tweaking code is disabled and compiled away.
//!
//! The `derive` feature exposes a proc macro to turn all literals from a function body into tweakable values.
//!
//! ## Usage
//!
//! ```rust
//! loop {
//!     // Try changing the value while the application is running
//!     println!("{}", inline_tweak::tweak!(3.14));
//! }
//! ```
//!
//! ## Extra features
//!
//! ### derive
//!
//! The `derive` feature allows to tweak any number/bool/char literal in a function.
//! It avoids cluttering the code with `inline_tweak::tweak!` calls.
//!
//! ```rust
//! #[inline_tweak::tweak_fn]
//! fn main() {
//!     loop {
//!        let v = 1.0; // Try changing this value!
//!        println!("{}", v);
//!        std::thread::sleep(std::time::Duration::from_millis(200)); // or even this value :)
//!     }
//! }
//! ```
//!
//! #### watch!
//!
//! `inline_tweak` provides a `watch!()` macro that sleeps until the file is modified, akin to a breakpoint:
//! ```rust
//! loop {
//!     println!("{}", inline_tweak::tweak!(3.14));
//!     watch!(); // The thread will sleep here until anything in the file changes
//! }
//! ```
//!
//! #### Expressions
//!
//! `inline_tweak` allows to tweak expressions by providing a value later.
//! For example:
//! ```rust
//! tweak!(rng.gen_range(0.0, 1.0))
//! ```
//!
//! can then be replaced by a constant value by modifying the file (even while the application is running) to
//! ```rust
//! tweak!(5.0; rng.gen_range(0.0, 1.0)) // will always return 5.0
//! ```
//!
//! #### release_tweak!
//!
//! The `release_tweak!` macro acts exactly like `tweak!` except that it also works in release mode.
//! It is accessible behind the feature flag `"release_tweak"` which is not enabled by default.
#![allow(clippy::needless_doctest_main)]

#[cfg(any(debug_assertions, feature = "release_tweak"))]
mod hasher;

pub trait Tweakable: Sized + Send + Clone + 'static {
    fn parse(x: &str) -> Option<Self>;
}

#[cfg(any(debug_assertions, feature = "release_tweak"))]
mod itweak {
    use super::Tweakable;
    use core::str::FromStr;
    use crate::hasher::FxHashMap;
    use std::any::Any;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::sync::{LazyLock, Mutex};
    use std::time::{Instant, SystemTime};

    macro_rules! impl_tweakable_float {
        ($($t: ty) +) => {
            $(
            impl Tweakable for $t {
                fn parse(x: &str) -> Option<$t> {
                    let v = x.replace("_", "");
                    FromStr::from_str(&v).ok()
                }
            }
            )+
        };
    }

    // Follows reference https://doc.rust-lang.org/reference/expressions/literal-expr.html
    macro_rules! impl_tweakable_integer {
        ($($t: ty) +) => {
            $(
            impl Tweakable for $t {
                fn parse(x: &str) -> Option<$t> {
                    let s = x.replace("_", "");
                    let radix = if s.starts_with("0x") {
                        16
                    } else if s.starts_with("0o") {
                        8
                    } else if s.starts_with("0b") {
                        2
                    } else {
                        10
                    };

                    let s_without_radix = if radix == 10 {
                        &s
                    } else {
                        &s[2..]
                    };

                    let v = i128::from_str_radix(&s_without_radix, radix).ok()?;

                    Some(v as $t)
                }
            }
            )+
        };
    }

    impl_tweakable_integer!(u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 usize isize);
    impl_tweakable_float!(f32 f64);

    impl Tweakable for bool {
        fn parse(x: &str) -> Option<Self> {
            match x {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        }
    }

    impl Tweakable for char {
        fn parse(x: &str) -> Option<Self> {
            x.trim_start_matches('\'')
                .trim_end_matches('\'')
                .chars()
                .next()
        }
    }

    impl Tweakable for &'static str {
        fn parse(x: &str) -> Option<Self> {
            let raw_remove = x.trim_start_matches(['r', '#']).trim_end_matches('#');
            let remove_starting_quote = raw_remove
                .split_once('"')
                .map(|v| v.1)
                .unwrap_or(raw_remove);

            let remove_ending_quote = remove_starting_quote
                .rsplit_once('"')
                .map(|v| v.0)
                .unwrap_or(remove_starting_quote);

            Some(Box::leak(Box::new(String::from(remove_ending_quote))))
        }
    }

    impl Tweakable for () {
        fn parse(_x: &str) -> Option<Self> {
            Some(())
        }
    }

    /// The struct holding the value of a call to tweak!.
    struct TweakValue {
        /// The value of the tweak. Downcasted to the type of the tweak when appropriate.
        value: Option<Box<dyn Any + Send>>,
        /// The last time this value was checked for modifications. Avoids too many hashmap lookups.
        last_checked: Instant,
        /// The version of the file when the value was last updated.
        file_version: u64,
    }

    /// A cache of the values of the tweaks in a file before being parsed.
    struct ParsedFile {
        /// The last time the file was checked for modifications. Avoids too many syscalls.
        last_checked_modified_time: Instant,
        file_modified: SystemTime,
        /// The list of the literal strings.
        values: Vec<String>,
        version: u64,
        /// The map of (line, column) -> position.
        /// This is only done once per file.
        /// This allows the line/columns to change without breaking the tweak.
        positions: Option<FxHashMap<(u32, u32), u32>>,
    }

    #[allow(dead_code)]
    struct FileWatcher {
        last_checked: Instant,
        file_modified: SystemTime,
    }

    #[derive(Hash, PartialEq, Eq)]
    struct TweakKey {
        filename: Filename,
        line: u32,
        column: u32,
    }

    type Filename = &'static str;

    /// Stores the values of the tweaks. The key is the file, line and column of the tweak.
    static VALUES: LazyLock<Mutex<FxHashMap<TweakKey, TweakValue>>> =
        LazyLock::new(Default::default);

    static PARSED_FILES: LazyLock<Mutex<FxHashMap<Filename, ParsedFile>>> =
        LazyLock::new(Default::default);

    static WATCHERS: LazyLock<Mutex<FxHashMap<Filename, FileWatcher>>> =
        LazyLock::new(Default::default);

    fn last_modified(file: Filename) -> Option<SystemTime> {
        File::open(file).ok()?.metadata().ok()?.modified().ok()
    }

    // Assume that the first time a tweak! is called, all tweak!s will be in original line/column.
    fn parse_tweak_positions(file: &mut ParsedFile, filename: Filename) -> Option<()> {
        let mut tweaks_seen = 0u32;

        let mut positions = FxHashMap::default();
        for (line_n, line) in BufReader::new(File::open(filename).ok()?)
            .lines()
            .map_while(Result::ok)
            .enumerate()
        {
            for (column, _) in line.match_indices("tweak!(") {
                let path_corrected_column = line[..column]
                    .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == ':' || c == '_')) // https://doc.rust-lang.org/reference/paths.html follows the rust path grammar
                    .map(|x| x + 1)
                    .unwrap_or(0);

                positions.insert(
                    (line_n as u32 + 1, path_corrected_column as u32 + 1),
                    tweaks_seen,
                );
                tweaks_seen += 1;
            }
        }

        file.positions = Some(positions);

        Some(())
    }

    fn parse_tweaks(f: &mut ParsedFile, filename: Filename) -> Option<()> {
        if f.last_checked_modified_time.elapsed() < std::time::Duration::from_millis(500)
            && f.version != 0
        {
            return Some(());
        }
        f.last_checked_modified_time = Instant::now();

        let last_modified = last_modified(filename).unwrap_or_else(SystemTime::now);

        if last_modified == f.file_modified && f.version != 0 {
            return Some(());
        }
        f.file_modified = last_modified;
        f.version += 1;

        f.values.clear();

        let content = std::fs::read_to_string(filename).ok()?;
        let mut it = content.split("tweak!(");

        it.next(); // skip part before first tweak!

        for val_str in it {
            // Find end of tweak
            let mut prec = 1;
            let (end, _) = val_str.char_indices().find(|(_, c)| {
                match c {
                    ';' | ')' if prec == 1 => {
                        return true;
                    }
                    ')' => prec -= 1,
                    '(' => prec += 1,
                    _ => {}
                }
                false
            })?;

            f.values.push(val_str[..end].to_string());
        }

        Some(())
    }

    fn update_tweak<T: Tweakable>(
        tweak: &mut TweakValue,
        line: u32,
        column: u32,
        file: &ParsedFile,
    ) -> Option<()> {
        if tweak.file_version == file.version {
            return Some(());
        }

        let position = file.positions.as_ref()?.get(&(line, column))?;

        let value = &**file.values.get(*position as usize)?;

        let parsed: Option<T> = Tweakable::parse(value);

        tweak.value = parsed.map(|inner| Box::new(inner) as Box<dyn Any + Send>);
        tweak.file_version = file.version;

        Some(())
    }

    pub(crate) fn get_value<T: Tweakable>(
        initial_value: Option<T>,
        filename: Filename,
        line: u32,
        column: u32,
    ) -> Option<T> {
        let mut lock = VALUES.lock().unwrap();

        let tweak = lock
            .entry(TweakKey {
                filename,
                line,
                column,
            })
            .or_insert_with(|| TweakValue {
                value: initial_value.map(|inner| Box::new(inner) as Box<dyn Any + Send>),
                last_checked: Instant::now(),
                file_version: 0,
            });

        if tweak.last_checked.elapsed().as_secs_f32() > 0.5 {
            tweak.last_checked = Instant::now();
            let mut fileinfos = PARSED_FILES.lock().unwrap();
            let f = fileinfos.entry(filename).or_insert_with(|| ParsedFile {
                last_checked_modified_time: Instant::now(),
                file_modified: SystemTime::now(),
                values: Default::default(),
                version: 0,
                positions: Default::default(),
            });

            if f.positions.is_none() {
                parse_tweak_positions(f, filename)?;
            }

            parse_tweaks(f, filename)?;

            update_tweak::<T>(tweak, line, column, f)?;
        }

        tweak.value.as_ref()?.downcast_ref().cloned()
    }

    #[allow(dead_code)]
    pub fn watch_modified(file: Filename) -> bool {
        let mut lock = WATCHERS.lock().unwrap();
        let entry = lock.entry(file);

        let now = Instant::now();

        let watcher = entry.or_insert_with(|| FileWatcher {
            last_checked: now,
            file_modified: last_modified(file).unwrap_or_else(SystemTime::now),
        });

        watcher.last_checked = now;

        let last_modified = last_modified(file).unwrap_or_else(SystemTime::now);
        last_modified
            .duration_since(watcher.file_modified)
            .map(|time| {
                watcher.file_modified = last_modified;
                time.as_secs_f32() > 0.5
            })
            .unwrap_or(true)
    }

    #[cfg(feature = "derive")]
    pub(crate) mod derive {
        use super::*;

        use crate::Tweakable;
        use crate::hasher::FxHashMap;
        use std::any::Any;
        use std::hash::{Hash, Hasher};
        use std::sync::Mutex;
        use std::time::{Instant, SystemTime};
        use syn::spanned::Spanned;
        use syn::visit::Visit;
        use syn::{
            Attribute, ExprConst, ImplItemFn, ItemConst, ItemFn, ItemStatic, Lit, TraitItemFn, Type,
        };

        struct ParsedFile {
            /// The last time the file was checked for modifications. Avoids too many syscalls.
            last_checked_modified_time: Instant,
            file_modified: SystemTime,
            /// Map of function name to the literal strings
            values: FxHashMap<String, Vec<String>>,
            version: u64,
        }

        /// Stores the values of the tweaks. The key is the file, the function name and the nth tweak
        /// within the function it is derived from.
        static VALUES_DERIVE: LazyLock<Mutex<FxHashMap<DeriveValueKey, TweakValue>>> =
            LazyLock::new(Default::default);

        /// Caches the values of the tweaks before being parsed.
        /// This allows only parsing the updated file once instead of every tweak call.
        static PARSED_DERIVE_VALUES: LazyLock<Mutex<FxHashMap<Filename, ParsedFile>>> =
            LazyLock::new(Default::default);

        #[derive(Debug, Hash, PartialEq, Eq)]
        struct DeriveValueKey {
            filename: Filename,
            nth: u32,
            fname_hash: u64, // Store a hash of the function name to avoid borrowing constraints
        }

        /// Visiter that finds all number/bool/char literals in a function.
        struct LiteralFinder<'a> {
            file: &'a mut ParsedFile,
            inside_derive_fn: Option<String>,
            derive_fn_count: u32,
        }

        impl<'a> LiteralFinder<'a> {
            fn enter_fn(
                &mut self,
                fn_name: String,
                attrs: &[Attribute],
                f: impl FnOnce(&mut Self),
            ) {
                let was_inside_derive_fn = self.inside_derive_fn.take();
                let was_derive_fn_count = self.derive_fn_count;
                if attrs.iter().any(|attr| {
                    attr.path()
                        .segments
                        .last()
                        .map(|seg| seg.ident == "tweak_fn" || seg.ident == "release_tweak_fn")
                        .unwrap_or(false)
                }) {
                    self.inside_derive_fn = Some(fn_name);
                    self.derive_fn_count = 0;
                }
                f(self);
                self.inside_derive_fn = was_inside_derive_fn;
                self.derive_fn_count = was_derive_fn_count;
            }
        }

        impl<'a, 'ast> Visit<'ast> for LiteralFinder<'a> {
            fn visit_impl_item_fn(&mut self, i: &'ast ImplItemFn) {
                self.enter_fn(i.sig.ident.to_string(), &i.attrs, |me| {
                    syn::visit::visit_impl_item_fn(me, i);
                });
            }

            fn visit_item_fn(&mut self, i: &'ast ItemFn) {
                self.enter_fn(i.sig.ident.to_string(), &i.attrs, |me| {
                    syn::visit::visit_item_fn(me, i);
                });
            }

            fn visit_lit(&mut self, l: &'ast Lit) {
                match l {
                    Lit::Char(_) | Lit::Int(_) | Lit::Float(_) | Lit::Bool(_) | Lit::Str(_) => {}
                    _ => return,
                }

                if let Some(ref fn_name) = self.inside_derive_fn {
                    if let Some(mut t) = l.span().source_text() {
                        let newlen = t.trim_end_matches(l.suffix()).len();
                        t.truncate(newlen);
                        if let Some(v) = self.file.values.get_mut(fn_name) {
                            v.push(t);
                        } else {
                            self.file.values.insert(fn_name.clone(), vec![t]);
                        }
                    }
                    self.derive_fn_count += 1;
                }
            }

            fn visit_expr(&mut self, i: &'ast syn::Expr) {
                match i {
                    syn::Expr::Unary(syn::ExprUnary {
                        op: syn::UnOp::Neg(_),
                        expr,
                        ..
                    }) => {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: l @ (Lit::Int(_) | Lit::Float(_)),
                            ..
                        }) = &**expr
                        {
                            if let Some(ref fn_name) = self.inside_derive_fn {
                                if let Some(mut t) = i.span().source_text() {
                                    let newlen = t.trim_end_matches(l.suffix()).len();
                                    t.truncate(newlen);
                                    if let Some(v) = self.file.values.get_mut(fn_name) {
                                        v.push(t);
                                    } else {
                                        self.file.values.insert(fn_name.clone(), vec![t]);
                                    }
                                }
                                self.derive_fn_count += 1;
                            }
                        } else {
                            syn::visit::visit_expr(self, i)
                        }
                    }
                    _ => syn::visit::visit_expr(self, i),
                }
            }

            fn visit_trait_item_fn(&mut self, i: &'ast TraitItemFn) {
                self.enter_fn(i.sig.ident.to_string(), &i.attrs, |me| {
                    syn::visit::visit_trait_item_fn(me, i);
                });
            }

            fn visit_attribute(&mut self, _: &Attribute) {}

            fn visit_expr_const(&mut self, _: &ExprConst) {}

            fn visit_item_const(&mut self, _: &ItemConst) {}

            fn visit_item_static(&mut self, _: &ItemStatic) {}

            fn visit_type(&mut self, _: &Type) {}
        }

        fn parse_tweaks_derive<'a>(f: &mut ParsedFile, filename: Filename) -> Option<()> {
            if f.last_checked_modified_time.elapsed() < std::time::Duration::from_millis(500)
                && f.version != 0
            {
                return Some(());
            }

            f.last_checked_modified_time = Instant::now();
            let last_modified = last_modified(filename).unwrap_or_else(SystemTime::now);

            if last_modified == f.file_modified {
                return Some(());
            }
            f.file_modified = last_modified;

            let content = std::fs::read_to_string(filename).ok()?;
            let parsed = syn::parse_file(&content).ok()?;

            f.values.clear();
            LiteralFinder {
                inside_derive_fn: None,
                file: f,
                derive_fn_count: 0,
            }
            .visit_file(&parsed);

            f.version += 1;

            Some(())
        }

        pub(crate) fn get_value_derive<T: Tweakable>(
            filename: Filename,
            function_name: &'static str,
            nth: u32,
        ) -> Option<T> {
            let mut lock = VALUES_DERIVE.lock().unwrap();

            let tweak = lock
                .entry(DeriveValueKey {
                    filename: filename,
                    nth,
                    fname_hash: {
                        let mut hasher = crate::hasher::FxHasher::default();
                        function_name.hash(&mut hasher);
                        hasher.finish()
                    },
                })
                .or_insert_with(|| TweakValue {
                    value: None,
                    last_checked: Instant::now(),
                    file_version: 0,
                });

            if tweak.last_checked.elapsed().as_secs_f32() > 0.5 {
                tweak.last_checked = Instant::now();
                let mut fileinfos = PARSED_DERIVE_VALUES.lock().unwrap();
                let f = fileinfos.entry(filename).or_insert_with(|| ParsedFile {
                    last_checked_modified_time: Instant::now(),
                    file_modified: SystemTime::now(),
                    values: Default::default(),
                    version: 0,
                });

                parse_tweaks_derive(f, filename)?;

                update_tweak_derive::<T>(tweak, function_name, nth, f)?;
            }

            tweak.value.as_ref()?.downcast_ref().cloned()
        }

        fn update_tweak_derive<T: Tweakable>(
            tweak: &mut TweakValue,
            function_name: &'static str,
            nth: u32,
            file: &ParsedFile,
        ) -> Option<()> {
            if tweak.file_version == file.version {
                return Some(());
            }

            let value = &**file.values.get(function_name)?.get(nth as usize)?;

            let parsed: Option<T> = Tweakable::parse(value);

            tweak.value = parsed.map(|inner| Box::new(inner) as Box<dyn Any + Send>);
            tweak.file_version = file.version;

            Some(())
        }
    }
}

#[cfg(any(debug_assertions, feature = "release_tweak"))]
pub fn inline_tweak<T: Tweakable>(
    initial_value: Option<T>,
    filename: &'static str,
    line: u32,
    column: u32,
) -> Option<T> {
    itweak::get_value(initial_value, filename, line, column)
}

#[cfg(all(feature = "derive", any(debug_assertions, feature = "release_tweak")))]
pub fn inline_tweak_derive<T: Tweakable>(
    file: &'static str,
    function_name: &'static str,
    nth: u32,
) -> Option<T> {
    itweak::derive::get_value_derive(file, function_name, nth)
}

#[cfg(all(feature = "release_tweak", not(target_arch = "wasm32")))]
mod macros_release {
    #[macro_export]
    macro_rules! release_tweak {
        ($default:expr) => {
            inline_tweak::inline_tweak(None, file!(), line!(), column!())
                .unwrap_or_else(|| $default)
        };
        ($value:literal; $default:expr) => {
            inline_tweak::inline_tweak(Some($value), file!(), line!(), column!())
                .unwrap_or_else(|| $default)
        };
    }

    #[macro_export]
    macro_rules! derive_release_tweak {
        ($default:expr, $fn_name:expr, $position:expr) => {
            inline_tweak::inline_tweak_derive(file!(), $fn_name, $position).unwrap_or($default)
        };
    }
}

#[cfg(all(feature = "release_tweak", target_arch = "wasm32"))]
mod macros_release {
    #[macro_export]
    macro_rules! release_tweak {
        ($default:expr) => {
            $default
        };
        ($value:literal; $default:expr) => {
            $default
        };
    }

    #[macro_export]
    macro_rules! derive_release_tweak {
        ($default:expr, $fn_name:expr, $position:expr) => {
            inline_tweak::inline_tweak_derive(file!(), $fn_name, $position)
                .unwrap_or_else(|| $default)
        };
    }
}

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
mod macros_tweak {
    use crate::itweak;
    #[macro_export]
    macro_rules! tweak {
        ($default:expr) => {
            inline_tweak::inline_tweak(None, file!(), line!(), column!())
                .unwrap_or_else(|| $default)
        };
        ($value:literal; $default:expr) => {
            inline_tweak::inline_tweak(Some($value), file!(), line!(), column!())
                .unwrap_or_else(|| $default)
        };
    }

    #[cfg(feature = "derive")]
    #[doc(hidden)]
    #[macro_export]
    macro_rules! derive_tweak {
        ($default:expr, $fn_name:expr, $position:expr) => {
            inline_tweak::inline_tweak_derive(file!(), $fn_name, $position).unwrap_or($default)
        };
    }

    #[cfg(not(feature = "derive"))]
    #[doc(hidden)]
    #[macro_export]
    macro_rules! derive_tweak {
        ($default:expr, $fn_name:expr, $position:expr) => {
            $default
        };
    }

    #[doc(hidden)]
    pub fn watch_file(filename: &'static str) {
        while !itweak::watch_modified(filename) {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    #[macro_export]
    macro_rules! watch {
        () => {
            inline_tweak::watch_file(file!());
        };
    }
}

#[cfg(any(not(debug_assertions), target_arch = "wasm32"))]
mod macros_tweak {
    #[macro_export]
    macro_rules! tweak {
        ($default:expr) => {
            $default
        };
        ($value:literal; $default:expr) => {
            $default
        };
    }

    #[macro_export]
    #[doc(hidden)]
    macro_rules! derive_tweak {
        ($default:expr, $fn_name:expr, $position:expr) => {
            $default
        };
    }

    #[doc(hidden)]
    pub fn watch_file(_filename: &'static str) {}

    #[macro_export]
    macro_rules! watch {
        () => {};
    }
}

pub use macros_tweak::*;

#[cfg(feature = "derive")]
pub use inline_tweak_derive::*;

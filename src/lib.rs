#[cfg(debug_assertions)]
mod itweak {
    use lazy_static::*;
    use std::any::Any;
    use std::collections::{HashMap, HashSet};
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::str::FromStr;
    use std::sync::Mutex;
    use std::time::{Instant, SystemTime};

    struct TweakValue {
        position: usize,
        value: Option<Box<dyn Any + Send>>,
        initialized: bool,
        last_checked: Instant,
        file_modified: SystemTime,
    }

    struct FileWatcher {
        last_checked: Instant,
        file_modified: SystemTime,
    }

    lazy_static! {
        static ref VALUES: Mutex<HashMap<(&'static str, u32, u32), TweakValue>> =
            Default::default();
        static ref PARSED_FILES: Mutex<HashSet<&'static str>> = Default::default();
        static ref WATCHERS: Mutex<HashMap<&'static str, FileWatcher>> = Default::default();
    }

    fn last_modified(file: &'static str) -> Option<SystemTime> {
        File::open(file).ok()?.metadata().ok()?.modified().ok()
    }

    // Assume that the first time a tweak! is called, all tweak!s will be in original position.
    fn parse_tweaks(file: &'static str) -> Option<()> {
        let mut fileinfos = PARSED_FILES.lock().unwrap();

        if !fileinfos.contains(&file) {
            let mut values = VALUES.lock().unwrap();

            let file_modified = last_modified(file).unwrap_or_else(SystemTime::now);
            let now = Instant::now();

            let mut tweaks_seen = 0;
            for (line_n, line) in BufReader::new(File::open(file).ok()?)
                .lines()
                .filter_map(|line| line.ok())
                .enumerate()
            {
                for (column, _) in line.match_indices("tweak!(") {
                    let path_corrected_column = line[..column]
                        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == ':' || c == '_')) // https://doc.rust-lang.org/reference/paths.html follows the rust path grammar
                        .map(|x| x + 1)
                        .unwrap_or(0);

                    values.insert(
                        (file, line_n as u32 + 1, path_corrected_column as u32 + 1),
                        TweakValue {
                            position: tweaks_seen,
                            value: None,
                            initialized: false,
                            last_checked: now,
                            file_modified,
                        },
                    );
                    tweaks_seen += 1;
                }
            }

            fileinfos.insert(file);
        }

        Some(())
    }

    fn update_tweak<T: 'static + FromStr + Clone + Send>(
        tweak: &mut TweakValue,
        file: &'static str,
    ) -> Option<()> {
        let last_modified = last_modified(file)?;
        if tweak.value.is_none()
            || last_modified
                .duration_since(tweak.file_modified)
                .ok()?
                .as_secs_f32()
                > 0.5
        {
            let mut tweaks_seen = 0;
            let line_str = BufReader::new(File::open(file).ok()?)
                .lines()
                .filter_map(|line| line.ok())
                .find(|line| {
                    tweaks_seen += line.matches("tweak!(").count();
                    tweaks_seen > tweak.position
                })?;
            let val_str = line_str
                .rsplit("tweak!(")
                .nth(tweaks_seen - tweak.position - 1)?;

            // Find end of tweak
            let mut prec = 1;
            let end = val_str.chars().position(|c| {
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

            let parsed: Option<T> = FromStr::from_str(&val_str[..end]).ok();
            tweak.file_modified = last_modified;
            tweak.last_checked = Instant::now();
            tweak.value = parsed.map(|inner| Box::new(inner) as Box<dyn Any + Send>);
        }

        Some(())
    }

    pub(crate) fn get_value<T: 'static + FromStr + Clone + Send>(
        initial_value: Option<T>,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> Option<T> {
        parse_tweaks(file);

        let mut lock = VALUES.lock().unwrap();
        let mut tweak = lock.get_mut(&(file, line, column))?;

        if !tweak.initialized {
            tweak.value = initial_value.map(|inner| Box::new(inner) as Box<dyn Any + Send>);
            tweak.initialized = true;
        }

        if Instant::now().duration_since(tweak.last_checked).as_secs_f32() > 0.5 {
            update_tweak::<T>(&mut tweak, file)?;
        }

        tweak.value.as_ref()?.downcast_ref().cloned()
    }

    pub fn watch_modified(file: &'static str) -> bool {
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
}

#[cfg(debug_assertions)]
pub fn inline_tweak<T: 'static + std::str::FromStr + Clone + Send>(
    initial_value: Option<T>,
    file: &'static str,
    line: u32,
    column: u32,
) -> Option<T> {
    itweak::get_value(initial_value, file, line, column)
}

#[cfg(debug_assertions)]
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

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! tweak {
    ($default:expr) => {
        $default
    };
    ($value:literal; $default:expr) => {
        $default
    };
}

#[cfg(debug_assertions)]
pub fn watch_file(file: &'static str) {
    while !itweak::watch_modified(file) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[cfg(not(debug_assertions))]
pub fn watch_file(_file: &'static str) {}

#[macro_export]
macro_rules! watch {
    () => {
        inline_tweak::watch_file(file!());
    };
}

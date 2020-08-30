#[cfg(debug_assertions)]
mod itweak {
    use lazy_static::*;
    use std::any::Any;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::str::FromStr;
    use std::sync::Mutex;
    use std::time::{Instant, SystemTime};

    struct TweakValue {
        last_checked: Instant,
        file_modified: SystemTime,
        value: Box<dyn Any + Send>,
    }

    lazy_static! {
        static ref VALUES: Mutex<HashMap<(&'static str, u32), TweakValue>> =
            Mutex::new(HashMap::new());
    }

    fn last_modified(file: &'static str) -> Option<SystemTime> {
        File::open(file).ok()?.metadata().ok()?.modified().ok()
    }

    pub(crate) fn get_value<T: 'static + FromStr + Clone + Send>(
        default: T,
        file: &'static str,
        line: u32,
    ) -> Option<T> {
        let mut lock = VALUES.lock().unwrap();
        let entry = lock.entry((file, line));

        let now = Instant::now();

        let tweak = entry.or_insert_with(|| TweakValue {
            last_checked: now,
            file_modified: last_modified(file).unwrap_or(SystemTime::now()),
            value: Box::new(default.clone()),
        });

        if now.duration_since(tweak.last_checked).as_secs_f32() < 0.5 {
            return tweak.value.downcast_ref().cloned();
        }

        tweak.last_checked = now;

        let last_modified = last_modified(file)?;
        if last_modified
            .duration_since(tweak.file_modified)
            .ok()?
            .as_secs_f32()
            > 0.5
        {
            let line_str = BufReader::new(File::open(file).ok()?)
                .lines()
                .nth((line - 1) as usize)?
                .ok()?;
            let val = line_str.find("tweak!(")? + 7; // 7 is length of "tweak!("
            let end = line_str[val..].find(')')? + val;
            let parsed: T = FromStr::from_str(&line_str[val..end]).ok()?;
            tweak.file_modified = last_modified;
            tweak.value = Box::new(parsed);
        }

        tweak.value.downcast_ref().cloned()
    }
}

#[cfg(debug_assertions)]
pub fn inline_tweak<T: 'static + std::str::FromStr + Clone + Send>(
    default: T,
    file: &'static str,
    line: u32,
) -> T {
    itweak::get_value(default.clone(), file, line).unwrap_or_else(move || default)
}

#[cfg(not(debug_assertions))]
pub fn inline_tweak<T: 'static + std::str::FromStr + Clone + Send>(
    default: T,
    _file: &'static str,
    _line: u32,
) -> T {
    default
}

#[macro_export]
macro_rules! tweak {
    ($e: literal) => {
        inline_tweak($e, file!(), line!())
    };
}

#[macro_use]
extern crate log;

extern crate high;
extern crate libloading;

use libloading::{Library, Symbol};
use std::{env, fs, mem, process, str, thread, time};
use std::borrow::Cow;

#[cfg(target_os = "macos")]
const DYNAMIC_LIBRARY_EXTENSION: &'static str = "dylib";
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "dragonfly"))]
const DYNAMIC_LIBRARY_EXTENSION: &'static str = "so";
#[cfg(target_os = "windows")]
const DYNAMIC_LIBRARY_EXTENSION: &'static str = "dll";

const DYNAMIC_LIBRARY_NAME: &'static str = concat!("lib", env!("CARGO_PKG_NAME"));
const SYMBOL: &'static [u8] = b"dyn_func";

fn main() {
	logger::init().expect("failed to initialize logger");

	high::currentize(load_then_drop);
}

pub fn load_then_drop() {

	let path_buf = {
		let exe = env::current_exe().unwrap();
		let directory = exe.parent().unwrap();
		directory.join(DYNAMIC_LIBRARY_NAME).with_extension(DYNAMIC_LIBRARY_EXTENSION)
	};

	let mut arguments = vec!["build"];

	if !cfg!(debug_assertions) { arguments.push("--release") }

	info!("Building project with `cargo build`");

	let output = process::
		Command::new("cargo")
				.args(&arguments)
				.output()
				.expect("failed to execute process");

	if output.status.success() {

		info!("Calling `fn`: {}", str::from_utf8(SYMBOL).unwrap());

		let library = Library::new(path_buf).expect("failed to load library");

		unsafe {

			let func: Symbol<fn() -> Result<(), Cow<'static, str>>> = 
				library.get(SYMBOL).expect("failed to get `fn`");

			if let Err(message) = func() {

				error!("{}", message);
			}
		}

		mem::drop(library);

	} else {

		error!("{}", output.status);
		error!("{}", String::from_utf8_lossy(&output.stdout));
		error!("{}", String::from_utf8_lossy(&output.stderr));

		wait_for_changes();
	}
}

fn wait_for_changes() {
	const SCRIPT_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/main.rs");

	info!("waiting for changes");

	let last_modified = fs::metadata(SCRIPT_PATH).unwrap().modified().unwrap();
	let dur = time::Duration::from_secs(2);

	loop {
		thread::sleep(dur);

		if let Ok(Ok(modified)) = fs::metadata(SCRIPT_PATH).map(|m| m.modified()) {

	        if modified > last_modified {
				break
	        }
	    }
	}
}

mod logger {

	use log::{self, LogRecord, LogLevel, LogLevelFilter, LogMetadata, SetLoggerError};

	pub struct Logger;

	impl log::Log for Logger {
	    fn enabled(&self, metadata: &LogMetadata) -> bool {
	        metadata.level() <= LogLevel::Info
	    }

	    fn log(&self, record: &LogRecord) {
	        if self.enabled(record.metadata()) {
	            println!("{} - {}", record.level(), record.args());
	        }
	    }
	}

	pub fn init() -> Result<(), SetLoggerError> {
	    log::set_logger(|max_log_level| {
	        max_log_level.set(LogLevelFilter::Info);
	        Box::new(Logger)
	    })
	}
}
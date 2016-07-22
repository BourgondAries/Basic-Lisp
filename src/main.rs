#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate isatty;
extern crate itertools;
#[macro_use]
extern crate slog;
extern crate slog_json;
extern crate slog_term;

use isatty::{stderr_isatty};
use itertools::Itertools;
use slog::*;
use std::collections::BTreeMap;
use std::io::{self, BufRead};

// Built-ins:
/*
	[fn]
	+
	-
	*
	/
	%
	@ (list)
	$ (variable)
*/
fn main() {

	let log = setup_logger(Level::Trace);

	log.trace("Acquiring stdin handle", b!("locked" => true));
	let stdin = io::stdin();
	let handle = stdin.lock();

	log.trace("Initializing interpreter", b!());
	let mut mach = Mach::new(&log);

	log.trace("Running interpreter", b!());
	handle
		.lines()
		.enumerate()
		.filter(|x| (x.1).is_ok())
		.map(|x| (x.0, (x.1).expect("Filter should have filtered this")))
		.inspect(|x| log.debug("Line", b!["length" => (x.1).len(), "line" => x.0, "content" => x.1]))
		.inspect(|x| { (x.1)
			.chars()
			.batching(|it| -> Option<()> {
				let mut word = String::new();
				for x in it {
					match x {
						'[' => mach.enter(),
						']' => mach.exit(),
						x if x.is_whitespace() => {
							mach.word(word);
							word = String::new();
						}
						_ => word.push(x),
					}
				}
				None
			})
			.count();
		})
		.count();
}

enum State {
	Inside(usize),
	Invalid,
	Outside,
}

impl State {
	fn new() -> State {
		State::Outside
	}
}

enum Sub {
	Tree(Vec<Sub>),
	Word(String),
}

struct Mach<'a> {
	macros: BTreeMap<String, i32>,
	state: State,
	log: &'a Logger,
	stack: Vec<Sub>,
}

impl<'a> Mach<'a> {
	fn new(log: &'a Logger) -> Mach {
		Mach {
			macros: BTreeMap::new(),
			state: State::new(),
			log: log,
			stack: vec![],
		}
	}

	fn word(&mut self, word: String) {

	}

	fn enter(&mut self) {
		self.state = match self.state {
			State::Inside(depth) => {
				self.log.trace("Incrementing inside", b!("depth" => depth + 1));
				State::Inside(depth + 1)
			}
			State::Invalid => {
				self.log.error("State invalid, should abort", b!());
				State::Invalid
			}
			State::Outside => {
				self.log.trace("Entering execution context", b!());
				State::Inside(0)
			}
		};
	}

	fn exit(&mut self) {
		self.state = match self.state {
			State::Inside(0) => {
				self.log.trace("Exiting execution context", b!());
				State::Outside
			}
			State::Invalid | State::Outside => {
				self.log.error("Invalid state", b!());
				State::Invalid
			}
			State::Inside(depth) => {
				self.log.trace("Inside state", b!("depth" => depth - 1));
				State::Inside(depth - 1)
			}
		};
	}
}

fn setup_logger(level: Level) -> Logger {
	let log = Logger::new_root(o!());

	if ! stderr_isatty() {
		log.set_drain(
			drain::filter_level(
				level,
				drain::async_stream(
					std::io::stderr(),
					slog_json::new(),
				),
			),
		);
		log.trace("Using json stderr drain", b!("stderr_isatty" => stderr_isatty(), "type" => "json"));
	} else {
		log.set_drain(
			drain::filter_level(
				level,
				slog_term::async_stderr()
			)
		);
		log.trace("Using terminal stderr drain", b!("stderr_isatty" => stderr_isatty(), "type" => "term"));
	}
	log
}


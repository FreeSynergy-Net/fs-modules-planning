//! The program `fs-modules-planning`: the service of the planning module. All of it is in the library.
//!
//! Pattern: Facade — one call.

fn main() {
    std::process::exit(fs_modules_planning::serve_command(std::env::args_os().skip(1)));
}

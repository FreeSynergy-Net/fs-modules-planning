//! The program `fs-modules-planning`: the service of the planning module. All of it is in the library.
//!
//! Pattern: Facade — one call, with the source of the local owner (`LocalOwner`, Strategy): the person "me" of the module
//! `person`, asked from its service. Without that person the service does not start and says why on stderr; no owner is made up.

fn main() {
    let running = std::env::current_exe().unwrap_or_default();
    let owner = fs_modules_planning::person_owner(&running);
    std::process::exit(fs_modules_planning::serve_command(
        std::env::args_os().skip(1),
        &*owner,
    ));
}

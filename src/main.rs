use errors::SsgError;
use jotdown::{Container, Event};
use std::{
    env,
    path::{Path, PathBuf},
};
use utils::{warn_or_error, warn_or_panic};
use walkdir::WalkDir;

use clap::Parser;

mod errors;
#[cfg(test)]
mod tests;
mod utils;

/// Djot static site generator
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ConsoleArgs {
    /// Path to the directory to use to generate the site
    target_path: PathBuf,
    /// Optional output path override. Defaults to ./output
    #[arg(short)]
    output_path: Option<PathBuf>,
    /// Clean the output directory before generating the site. Useful for multiple runs
    #[arg(long)]
    clean: bool,
    /// Disallow any warnings
    #[arg(long)]
    no_warn: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    log::trace!("Begin djot-ssg::main()");
    let args = ConsoleArgs::parse();
    run_program(args)
}

fn run_program(args: ConsoleArgs) -> anyhow::Result<()> {
    let output_path = args
        .output_path
        .unwrap_or(env::current_dir()?.join("output"));
    // Clean the output directory if clean is specified
    if args.clean {
        log::debug!(
            "Clean argument specified, cleaning output path {:?}...",
            &output_path
        );
        if let Err(_) = std::fs::remove_dir_all(&output_path) {
            log::trace!("Nothing to clean!");
        } else {
            log::trace!("Clean successful!");
        }
    }
    generate_site(&args.target_path, &output_path, args.no_warn)?;
    Ok(())
}

fn generate_site(target_path: &Path, output_path: &Path, no_warn: bool) -> anyhow::Result<()> {
    let _ = std::fs::create_dir_all(output_path);
    log::trace!(
        "Created output directory {:?} if it didn't exist...",
        output_path
    );
    if !utils::check_has_index(target_path) {
        warn_or_error(SsgError::IndexPageNotFound, no_warn)?;
    }
    for entry in WalkDir::new(target_path) {
        match entry {
            Ok(direntry) => {
                if direntry.path().is_dir() {
                    log::trace!("Path {:?} is a directory, continuing...", direntry.path());
                    continue;
                }
                log::trace!("Path: {:?}", direntry.path());
                let new_path = match utils::get_relative_path(direntry.path(), &target_path) {
                    Some(relative) => output_path.join(relative),
                    None => {
                        warn_or_error(
                            SsgError::PathNotRelative(direntry.path().to_path_buf()),
                            no_warn,
                        )?;
                        continue;
                    }
                };
                let _ = std::fs::create_dir_all(&new_path.parent().unwrap());
                match direntry.path().extension().map(|x| x.to_str().unwrap()) {
                    Some("dj") | Some("djot") => {
                        let result_path = new_path.with_extension("html");
                        log::debug!(
                            "Generating .html from {:?} and moving to {:?}",
                            direntry.path(),
                            &result_path
                        );
                        let djot_input = std::fs::read_to_string(direntry.path())?;
                        let html =
                            process_djot(&djot_input, direntry.path().parent().unwrap(), no_warn);
                        std::fs::write(&result_path, &html.as_bytes())?;
                    }
                    _ => {
                        std::fs::copy(direntry.path(), &new_path)?;
                    }
                }
            }
            Err(e) => {
                warn_or_error(SsgError::DirEntryError(e), no_warn)?;
            }
        }
    }
    Ok(())
}

fn process_djot(djot_input: &str, file_parent_dir: &Path, no_warn: bool) -> String {
    let events = jotdown::Parser::new(&djot_input).map(|event| match event {
        Event::Start(Container::Link(s, link_type), a) => {
            let inner = s.to_string();
            let referenced_path = file_parent_dir.join(s.to_string());
            let new_path = Path::new(&inner).with_extension("html");
            if referenced_path.exists() {
                Event::Start(
                    Container::Link(
                        std::borrow::Cow::Owned(new_path.to_string_lossy().to_string()),
                        link_type,
                    ),
                    a,
                )
            } else {
                warn_or_panic(SsgError::LinkError(referenced_path.clone()), no_warn);
                Event::Start(Container::Link(s, link_type), a)
            }
        }
        Event::End(Container::Link(s, link_type)) => {
            let inner = s.to_string();
            let referenced_path = file_parent_dir.join(s.to_string());
            let new_path = Path::new(&inner).with_extension("html");
            if referenced_path.exists() {
                Event::End(Container::Link(
                    std::borrow::Cow::Owned(new_path.to_string_lossy().to_string()),
                    link_type,
                ))
            } else {
                Event::End(Container::Link(s, link_type))
            }
        }
        _ => event,
    });
    for event in events.clone() {
        // log::trace!("EVENT: {:?}", event);
        match event {
            jotdown::Event::Start(jotdown::Container::Link(s, link_type), a) => {
                log::debug!(
                    "Link found! {} :: Type: {:?} :: Container attributes: {:?}",
                    s,
                    link_type,
                    a
                );
            }
            _ => {}
        }
    }
    let mut html = jotdown::html::render_to_string(events);
    // For now, just replace all .dj, .djot with .html
    html = html.replace(".dj", ".html");
    html = html.replace(".djot", ".html");
    html
}

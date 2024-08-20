use std::{
    env,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use clap::Parser;

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
    let output_path = args
        .output_path
        .unwrap_or(env::current_dir()?.join("output"));
    // Clean the output directory if clean is specified
    if args.clean {
        log::debug!(
            "Clean argument specified, cleaning output path {:?}...",
            &output_path
        );
        if let Err(e) = std::fs::remove_dir_all(&output_path) {
            log::warn!("Cleanup failed: {}", e);
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
        warn_or_error!(no_warn, "index.{{dj|djot}} not found! consider creating one in the base target directory as the default page.");
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
                        warn_or_error!(
                            no_warn,
                            "Path {:?} is not relative to {:?}, skipping...",
                            direntry.path(),
                            &target_path
                        );
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
                        let events = jotdown::Parser::new(&djot_input);
                        let html = jotdown::html::render_to_string(events);
                        std::fs::write(&result_path, &html.as_bytes())?;
                    }
                    _ => {
                        std::fs::copy(direntry.path(), &new_path)?;
                    }
                }
            }
            Err(e) => {
                warn_or_error!(no_warn, "An entry returned error {}", e);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn generate_sample_site() {}
}

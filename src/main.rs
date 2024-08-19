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
}

fn main() -> anyhow::Result<()> {
    log::trace!("Begin djot-ssg::main()");
    let args = ConsoleArgs::parse();
    generate_site(
        &args.target_path,
        &args
            .output_path
            .unwrap_or(env::current_dir()?.join("output")),
    )?;
    Ok(())
}

fn generate_site(target_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    let _ = std::fs::create_dir_all(output_path);
    for entry in WalkDir::new(target_path) {
        match entry {
            Ok(direntry) => {
                log::trace!("Path: {:?}", direntry.path());
                let new_path = match utils::get_relative_path(direntry.path(), &target_path) {
                    Some(relative) => output_path.join(relative),
                    None => {
                        log::warn!(
                            "Path {:?} is not relative to {:?}, skipping...",
                            direntry.path(),
                            &target_path
                        );
                        continue;
                    }
                };
                let _ = std::fs::create_dir_all(&new_path.parent().unwrap());
                match direntry.path().extension().map(|x| x.to_str().unwrap()) {
                    Some(".dj") | Some(".djot") => {
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
                log::warn!("An entry returned error {e}");
            }
        }
    }
    Ok(())
}

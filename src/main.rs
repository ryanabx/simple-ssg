use errors::SsgError;
use jotdown::{Container, Event};
use pulldown_cmark::CowStr;
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};
use utils::warn_or_error;
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
    /// Specify the website prefix (defaults to local paths i.e. `./`)
    #[arg(long)]
    web_prefix: Option<String>,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    log::trace!("Begin smpl-ssg::main()");
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
    generate_site(
        &args.target_path,
        &output_path,
        args.no_warn,
        args.web_prefix.as_deref(),
    )?;
    Ok(())
}

fn generate_site(
    target_path: &Path,
    output_path: &Path,
    no_warn: bool,
    web_prefix: Option<&str>,
) -> anyhow::Result<()> {
    let _ = std::fs::create_dir_all(output_path);
    log::trace!(
        "Created output directory {:?} if it didn't exist...",
        output_path
    );
    if !utils::check_has_index(target_path) {
        warn_or_error(SsgError::IndexPageNotFound, no_warn)?;
    }
    let mut html_results: HashMap<PathBuf, String> = HashMap::new();
    let mut table_of_contents_html = "<ul>".to_string();
    log::info!("1/3: Site generation and indexing...");
    for entry in WalkDir::new(target_path) {
        match entry {
            Ok(direntry) => {
                log::debug!("{:?}", direntry.path());
                if direntry.path().is_dir() {
                    log::trace!("Path {:?} is a directory, continuing...", direntry.path());
                    continue;
                } else if direntry.path().ends_with("template.html") {
                    log::trace!("Path {:?} is a template, continuing...", direntry.path());
                    continue;
                }
                log::trace!("Path: {:?}", direntry.path());
                let relative = match direntry.path().strip_prefix(target_path) {
                    Ok(relative) => relative.to_path_buf(),
                    Err(_) => {
                        warn_or_error(
                            SsgError::PathNotRelative(direntry.path().to_path_buf()),
                            no_warn,
                        )?;
                        continue;
                    }
                };
                let new_path = output_path.join(&relative);
                let _ = std::fs::create_dir_all(new_path.parent().unwrap());
                match direntry.path().extension().map(|x| x.to_str().unwrap()) {
                    Some("dj") | Some("djot") | Some("md") => {
                        let template = utils::get_template_if_exists(direntry.path(), target_path)?;
                        let result_path = new_path.with_extension("html");
                        log::debug!(
                            "Generating .html from {:?} and moving to {:?}",
                            direntry.path(),
                            &result_path
                        );
                        let input_str = std::fs::read_to_string(direntry.path())?;
                        let html = match direntry.path().extension().map(|x| x.to_str().unwrap()) {
                            Some("md") => process_markdown(
                                &input_str,
                                direntry.path().parent().unwrap(),
                                no_warn,
                                web_prefix,
                            )?,
                            Some("dj") | Some("djot") => process_djot(
                                &input_str,
                                direntry.path().parent().unwrap(),
                                no_warn,
                                web_prefix,
                            )?,
                            _ => unreachable!(),
                        };
                        let html_formatted = utils::wrap_html_content(&html, template.as_deref());
                        html_results.insert(result_path, html_formatted);
                        table_of_contents_html.push_str(&format!(
                            "<li><a href=\"{}{}\">{}</a></li>",
                            &web_prefix.unwrap_or("./"),
                            &relative.with_extension("html").to_string_lossy(),
                            &relative.with_extension("html").to_string_lossy()
                        ))
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
    table_of_contents_html.push_str("</ul>");
    // Validation pass
    log::info!("2/3: Generating additional site content (if necessary) and saving...");
    for (path, text) in html_results.iter() {
        let text = text.replace("<!-- {TABLE_OF_CONTENTS} -->", &table_of_contents_html);
        std::fs::write(path, text.as_bytes())?;
    }

    log::info!("3/3: Done!");

    Ok(())
}

fn process_markdown(
    markdown_input: &str,
    file_parent_dir: &Path,
    no_warn: bool,
    web_prefix: Option<&str>,
) -> anyhow::Result<String> {
    let events = pulldown_cmark::Parser::new(markdown_input)
        .map(|event| -> anyhow::Result<pulldown_cmark::Event> {
            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => {
                    let inner = dest_url.to_string();
                    let referenced_path = file_parent_dir.join(&inner);
                    if referenced_path
                        .extension()
                        .is_some_and(|ext| ext == "dj" || ext == "djot" || ext == "md")
                    {
                        let new_path = Path::new(&inner).with_extension("html");
                        if !referenced_path.exists() {
                            warn_or_error(SsgError::LinkError(referenced_path), no_warn)?;
                        }
                        let dest_url = CowStr::Boxed(
                            format!("{}{}", web_prefix.unwrap_or(""), new_path.to_string_lossy())
                                .into_boxed_str(),
                        );
                        Ok(pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link {
                            link_type,
                            dest_url,
                            title,
                            id,
                        }))
                    } else {
                        Ok(pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link {
                            link_type,
                            dest_url,
                            title,
                            id,
                        }))
                    }
                }
                _ => Ok(event),
            }
        })
        .collect::<Result<Vec<pulldown_cmark::Event>, _>>()?;

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.iter().cloned());
    Ok(html)
}

fn process_djot(
    djot_input: &str,
    file_parent_dir: &Path,
    no_warn: bool,
    web_prefix: Option<&str>,
) -> anyhow::Result<String> {
    let events = jotdown::Parser::new(djot_input)
        .map(|event| -> anyhow::Result<Event> {
            match event {
                Event::Start(Container::Link(text, link_type), attributes) => {
                    let inner = text.to_string();
                    let referenced_path = file_parent_dir.join(&inner);
                    if referenced_path
                        .extension()
                        .is_some_and(|ext| ext == "dj" || ext == "djot" || ext == "md")
                    {
                        let new_path = Path::new(&inner).with_extension("html");
                        if referenced_path.exists() {
                            Ok(Event::Start(
                                Container::Link(
                                    std::borrow::Cow::Owned(format!(
                                        "{}{}",
                                        web_prefix.unwrap_or(""),
                                        new_path.to_string_lossy()
                                    )),
                                    link_type,
                                ),
                                attributes,
                            ))
                        } else {
                            warn_or_error(SsgError::LinkError(referenced_path), no_warn)?;
                            Ok(Event::Start(Container::Link(text, link_type), attributes))
                        }
                    } else {
                        Ok(Event::Start(Container::Link(text, link_type), attributes))
                    }
                }
                Event::End(Container::Link(text, link_type)) => {
                    let inner = text.to_string();
                    let referenced_path = file_parent_dir.join(&inner);
                    if referenced_path
                        .extension()
                        .is_some_and(|ext| ext == "dj" || ext == "djot" || ext == "md")
                    {
                        let new_path = Path::new(&inner).with_extension("html");
                        if referenced_path.exists() {
                            Ok(Event::End(Container::Link(
                                std::borrow::Cow::Owned(format!(
                                    "{}{}",
                                    web_prefix.unwrap_or(""),
                                    new_path.to_string_lossy()
                                )),
                                link_type,
                            )))
                        } else {
                            Ok(Event::End(Container::Link(text, link_type)))
                        }
                    } else {
                        Ok(Event::End(Container::Link(text, link_type)))
                    }
                }
                _ => Ok(event),
            }
        })
        .collect::<Result<Vec<Event>, _>>()?;
    let html = jotdown::html::render_to_string(events.iter().cloned());
    Ok(html)
}

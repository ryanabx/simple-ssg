use anyhow::anyhow;
use errors::SsgError;
use jotdown::{Container, Event};
use pulldown_cmark::CowStr;
use std::{
    env,
    path::{Path, PathBuf},
};
use templates::BuiltInTemplate;
use walkdir::WalkDir;

use clap::Parser;

mod errors;
mod templates;
#[cfg(test)]
mod tests;
mod utils;

/// Djot static site generator
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ConsoleArgs {
    /// Path to the directory to use to generate the site (not required if -f is specified)
    directory: Option<PathBuf>,
    /// Process a single file instead of a directory
    #[arg(short, conflicts_with = "clean")]
    file: Option<PathBuf>,
    /// Optional output path override. Defaults to ./output for directories
    #[arg(short)]
    output_path: Option<PathBuf>,
    /// Clean the output directory before generating the site. Useful for multiple runs
    #[arg(long, conflicts_with = "file")]
    clean: bool,
    /// Specify the website prefix (defaults to local paths i.e. `./`)
    #[arg(long)]
    web_prefix: Option<String>,
    /// Specify a built in template to use (will override a template.html
    /// in any directory!). defaults to whatever templates are found in template.html in the
    /// directories.
    #[arg(short, long)]
    template: Option<BuiltInTemplate>,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    log::trace!("Begin simple-ssg::main()");
    let args = ConsoleArgs::parse();
    run_program(args)
}

fn run_program(args: ConsoleArgs) -> anyhow::Result<()> {
    let (target_path, output_path) = if args.directory.is_some() && args.file.is_some() {
        return Err(anyhow!(
            "Cannot specify both a directory and a path! (Specified {} and -f {})",
            args.directory.unwrap().display(),
            args.file.unwrap().display()
        ));
    } else if let Some(dir) = args.directory {
        if dir.is_file() {
            return Err(anyhow!(
                "Path {} is a file. Specify -f <FILE> if this was intended.",
                dir.display()
            ));
        }
        (dir, env::current_dir()?.join("output"))
    } else if let Some(path) = args.file {
        if path.is_dir() {
            return Err(anyhow!("Path {} is a directory. Specify <DIRECTORY> without the -f positional argument if this was intended.", path.display()));
        }
        (path, env::current_dir()?)
    } else {
        return Err(anyhow!(
            "Must specify either a directory <DIRECTORY> or a path with -f <PATH>"
        ));
    };
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
        &target_path,
        &output_path,
        args.web_prefix.as_deref(),
        args.template,
    )?;
    Ok(())
}

#[derive(Clone, Debug)]
pub enum FirstPassResult {
    Dir {
        depth: usize,
        relative_path: PathBuf,
    },
    HtmlOutput {
        depth: usize,
        html: String,
        relative_path: PathBuf,
    },
}

fn generate_site(
    target_path: &Path,
    output_path: &Path,
    web_prefix: Option<&str>,
    template: Option<BuiltInTemplate>,
) -> anyhow::Result<()> {
    let _ = std::fs::create_dir_all(output_path);
    log::trace!(
        "Created output directory {:?} if it didn't exist...",
        output_path
    );

    let mut first_pass_results = Vec::new();

    log::info!("1/3: Site generation and indexing...");
    if target_path.is_dir() && output_path.is_dir() {
        if !utils::check_has_index(target_path) {
            log::warn!("{}", SsgError::IndexPageNotFound);
        }
        for entry in WalkDir::new(target_path) {
            match entry {
                Ok(direntry) => process_path(
                    direntry.path(),
                    target_path,
                    output_path,
                    &template,
                    web_prefix,
                    direntry.depth(),
                    &mut first_pass_results,
                )?,
                Err(e) => {
                    log::warn!("{}", SsgError::DirEntryError(e));
                }
            }
        }
    } else if target_path.is_file() {
        process_path(
            target_path,
            target_path.parent().unwrap(),
            output_path,
            &template,
            web_prefix,
            1,
            &mut first_pass_results,
        )?;
    } else {
        return Err(anyhow!(
            "Target path {} is not a file or a directory.",
            target_path.display()
        ));
    }

    // Validation pass
    log::info!("2/3: Generating additional site content (if necessary) and saving...");

    for result in first_pass_results.clone() {
        match result {
            FirstPassResult::Dir { .. } => continue,
            FirstPassResult::HtmlOutput {
                depth,
                html,
                relative_path,
            } => {
                let table_of_contents = generate_table_of_contents(
                    &first_pass_results,
                    depth,
                    &relative_path,
                    web_prefix,
                );
                let text = html.replace("<!-- {TABLE_OF_CONTENTS} -->", &table_of_contents);
                let result_path = output_path.join(&relative_path);
                log::debug!("{:?} :: {:?}", &result_path, &relative_path);
                std::fs::write(&result_path, text.as_bytes())?;
            }
        }
        // Generate the table of contents
    }

    log::info!("3/3: Done!");

    Ok(())
}

fn process_path(
    entity: &Path,
    target_path: &Path,
    output_path: &Path,
    template: &Option<BuiltInTemplate>,
    web_prefix: Option<&str>,
    depth: usize,
    first_pass_results: &mut Vec<FirstPassResult>,
) -> anyhow::Result<()> {
    let relative = match entity.strip_prefix(target_path) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => {
            log::warn!("{}", SsgError::PathNotRelative(entity.to_path_buf()));
            return Ok(());
        }
    };
    log::debug!("{:?} :: {}", &relative, depth);
    if entity.is_dir() {
        log::trace!("Path {:?} is a directory, continuing...", entity);
        first_pass_results.push(FirstPassResult::Dir {
            depth,
            relative_path: relative,
        });
        return Ok(());
    } else if entity.ends_with("template.html") {
        log::trace!("Path {:?} is a template, continuing...", entity);
        return Ok(());
    }
    log::trace!("Path: {:?}", entity);
    let new_path = output_path.join(&relative);
    let _ = std::fs::create_dir_all(new_path.parent().unwrap());
    match entity.extension().map(|x| x.to_str().unwrap()) {
        Some("dj") | Some("djot") | Some("md") => {
            let html_template = template.clone().map_or(
                utils::get_template_if_exists(entity, target_path)?,
                |template| Some(template.get_template()),
            );
            let result_path = new_path.with_extension("html");
            log::debug!(
                "Generating .html from {:?} and moving to {:?}",
                entity,
                &result_path
            );
            let input_str = std::fs::read_to_string(entity)?;
            let html = match entity.extension().map(|x| x.to_str().unwrap()) {
                Some("md") => process_markdown(&input_str, entity.parent().unwrap(), web_prefix)?,
                Some("dj") | Some("djot") => {
                    process_djot(&input_str, entity.parent().unwrap(), web_prefix)?
                }
                _ => unreachable!(),
            };
            let html_formatted = utils::wrap_html_content(&html, html_template.as_deref());
            first_pass_results.push(FirstPassResult::HtmlOutput {
                depth: depth,
                html: html_formatted,
                relative_path: relative.with_extension("html"),
            });
        }
        _ => {
            std::fs::copy(entity, &new_path)?;
        }
    }
    Ok(())
}

fn process_markdown(
    markdown_input: &str,
    file_parent_dir: &Path,
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
                            log::warn!("{}", SsgError::LinkError(referenced_path))
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
                            log::warn!("{}", SsgError::LinkError(referenced_path));
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

fn generate_table_of_contents(
    results: &Vec<FirstPassResult>,
    my_depth: usize,
    my_result: &Path,
    web_prefix: Option<&str>,
) -> String {
    let mut table_of_contents_html = "<ul>".to_string();
    log::debug!("<ul>");
    let mut prev_depth = 0;
    let mut prev_file_depth = 0;
    let mut prev_folders = Vec::new();
    for result in results {
        match result {
            FirstPassResult::Dir {
                depth,
                relative_path,
            } => {
                log::trace!("Dir: {}", &relative_path.to_string_lossy());
                let mut depth_diff = *depth as i32 - prev_depth as i32;
                while depth_diff < 0 {
                    if prev_folders.pop().is_none() {
                        let format_string = format!("</ul>");
                        log::debug!("{} (Dir, depth_diff={})", &format_string, depth_diff);
                        table_of_contents_html.push_str(&format_string);
                    }
                    depth_diff += 1;
                }
                prev_depth = *depth;
                if *depth > 0 {
                    log::trace!(
                        "Adding {} to the folders stack (at depth {})",
                        &relative_path.to_string_lossy(),
                        *depth
                    );
                    prev_folders.push(
                        relative_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                    );
                }
            }
            FirstPassResult::HtmlOutput {
                relative_path,
                depth,
                ..
            } => {
                log::trace!("File: {}", &relative_path.to_string_lossy());
                let mut depth_diff = *depth as i32 - prev_depth as i32;
                while depth_diff < 0 {
                    if prev_folders.pop().is_none() {
                        let format_string = format!("</ul>");
                        log::debug!("{}  (File, depth_diff={})", &format_string, depth_diff);
                        table_of_contents_html.push_str(&format_string);
                    }
                    depth_diff += 1;
                }
                let mut pos_depth_diff = prev_folders.len();
                while pos_depth_diff > 0 {
                    let folder_name = prev_folders.remove(0);
                    let format_string = format!("<li><b><u>{}:</u></b></li>", &folder_name,);
                    log::debug!(
                        "{} (folder, depth={})",
                        &format_string,
                        (*depth - pos_depth_diff)
                    );
                    table_of_contents_html.push_str(&format_string);
                    let format_string = format!("<ul>");
                    log::debug!("{}, prev_folders-={}", &format_string, &folder_name);
                    table_of_contents_html.push_str(&format_string);
                    pos_depth_diff -= 1;
                }
                prev_depth = *depth;
                prev_file_depth = *depth;
                if relative_path == my_result {
                    let format_string = format!(
                        "<li><b>{}</b></li>",
                        &relative_path.file_stem().unwrap().to_string_lossy()
                    );
                    log::debug!("{} (file, depth={})", &format_string, *depth);
                    table_of_contents_html.push_str(&format_string);
                } else {
                    let format_string = format!(
                        "<li><a href=\"{}{}{}\">{}</a></li>",
                        if my_depth > 1 {
                            "../".repeat(my_depth - 1)
                        } else {
                            "".to_string()
                        },
                        &web_prefix.unwrap_or(""), // "./" if "" doesn't work
                        &relative_path.to_string_lossy(),
                        &relative_path.file_stem().unwrap().to_string_lossy()
                    );
                    log::debug!("{} (file, depth={})", &format_string, *depth);
                    table_of_contents_html.push_str(&format_string);
                }
            }
        }
    }
    prev_depth -= prev_folders.len();
    log::trace!("prev_depth - {} = {}", prev_folders.len(), prev_depth);
    log::trace!("prev_file_depth = {}", prev_file_depth);
    let mut depth_diff = 0 - prev_file_depth as i32;
    while depth_diff < 0 {
        let format_string = format!("</ul>");
        log::debug!("{} (end, depth_diff={})", &format_string, depth_diff);
        table_of_contents_html.push_str(&format_string);
        depth_diff += 1;
    }
    // log::debug!("Table of contents: {}", &table_of_contents_html);
    table_of_contents_html
}

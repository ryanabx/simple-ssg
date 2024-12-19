use pulldown_cmark::{CowStr, Options};
use std::{
    fs::{self, read_dir},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use clap::Parser;
#[cfg(test)]
mod tests;

/// Simple static site generator
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ConsoleArgs {
    /// Path to the directory to use to generate the site
    directory: PathBuf,
    /// Path of the base output directory
    output_path: PathBuf,
    /// Optional web prefix
    prefix: Option<String>,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    log::trace!("Begin simple-ssg::main()");
    let args = ConsoleArgs::parse();
    generate(&args.directory, &args.output_path, args.prefix.as_deref())?;
    Ok(())
}

fn generate(root_dir: &Path, output_dir: &Path, web_prefix: Option<&str>) -> anyhow::Result<()> {
    let root_dir = root_dir.canonicalize()?;
    let _ = fs::create_dir_all(&output_dir);
    let output_dir = output_dir.canonicalize()?;

    for x in WalkDir::new(&root_dir) {
        let x = x?;
        let x_path = x.path();
        log::info!("{:?}", x_path);
        if x_path.is_dir() {
            if !x_path.join("index.md").exists() {
                let mut directory_index = String::new();
                if x.depth() > 0 {
                    // let parent_dir = x_path.parent().unwrap();

                    directory_index.push_str("[../](../index.html)\n");
                    directory_index.push_str("\n");
                }
                directory_index.push_str(&create_directory_index(x_path)?);
                log::info!("{}", &directory_index);
                let html = md_to_html(&directory_index, x_path, web_prefix)?;
                let result_path =
                    output_dir.join(x_path.join("index.html").strip_prefix(&root_dir)?);
                log::info!("Result path: {:?}", &result_path);
                let _ = std::fs::create_dir_all(result_path.parent().unwrap());
                if let Err(e) = std::fs::write(&result_path, html.as_bytes()) {
                    log::error!("Could not write file to path {:?}: {}", &result_path, e);
                    anyhow::bail!(e);
                }
            }
        } else if x_path.is_file() {
            let result_path =
                output_dir.join(x_path.with_extension("html").strip_prefix(&root_dir)?);
            if x_path.extension().is_some_and(|ext| ext == "md") {
                let md = fs::read_to_string(x_path)?;
                let html = md_to_html(&md, x_path.parent().unwrap(), web_prefix)?;
                let _ = std::fs::create_dir_all(result_path.parent().unwrap());
                if let Err(e) = std::fs::write(&result_path, html.as_bytes()) {
                    log::error!("Could not write file to path {:?}: {}", &result_path, e);
                    anyhow::bail!(e);
                }
            } else {
                let _ = std::fs::create_dir_all(result_path.parent().unwrap());
                if let Err(e) = std::fs::copy(x_path, &result_path) {
                    log::error!(
                        "Could not copy file from {:?} to {:?}: {}",
                        x_path,
                        &result_path,
                        e
                    );
                    anyhow::bail!(e);
                }
            }
        }
    }

    Ok(())
}

fn get_relative_url(root_path: &Path, target_path: &Path) -> String {
    log::info!("{:?}.strip({:?})", target_path, root_path);
    target_path
        .strip_prefix(root_path)
        .unwrap()
        .to_string_lossy()
        .to_string()
}

fn create_directory_index(folder: &Path) -> anyhow::Result<String> {
    let mut result = String::new();

    for x in read_dir(folder)? {
        let x_path = &x?.path();
        result.push_str(&directory_string(
            folder,
            &x_path,
            &x_path.file_name().unwrap().to_string_lossy(),
        ));
        result.push_str("\n");
    }

    Ok(result)
}

fn directory_string(root_path: &Path, target_path: &Path, name: &str) -> String {
    format!(
        "[{}](./{})\n",
        name,
        get_relative_url(root_path, target_path)
    )
}

/// Take markdown input, and change links if necessary
pub fn md_to_html(
    markdown: &str,
    file_parent_dir: &Path,
    web_prefix: Option<&str>,
) -> anyhow::Result<String> {
    log::info!("Markdown to html: {:?}", file_parent_dir);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let events = pulldown_cmark::Parser::new_ext(markdown, options)
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
                    if referenced_path.extension().is_some_and(|ext| ext == "md") {
                        let new_path = Path::new(&inner).with_extension("html");
                        log::info!("Path: {:?} -> {:?}", &inner, &new_path);
                        if !referenced_path.exists() {
                            log::warn!("Path {:?} doesn't exist!", referenced_path);
                            Ok(pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link {
                                link_type,
                                dest_url,
                                title,
                                id,
                            }))
                        } else {
                            // Create destination url
                            let dest_url = CowStr::Boxed(
                                format!(
                                    "{}{}",
                                    web_prefix.unwrap_or(""),
                                    new_path.to_string_lossy()
                                )
                                .into_boxed_str(),
                            );
                            Ok(pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link {
                                link_type,
                                dest_url,
                                title,
                                id,
                            }))
                        }
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
    log::info!("Done");
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.iter().cloned());
    Ok(html)
}

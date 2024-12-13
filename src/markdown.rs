use std::path::Path;

use pulldown_cmark::{CowStr, Options};


/// Take markdown input, and change links if necessary
pub fn md_to_html(
    markdown: &str,
    file_parent_dir: &Path,
    web_prefix: Option<&str>,
) -> anyhow::Result<String> {
    log::info!("Markdown to html: {:?}", file_parent_dir);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
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
                    if referenced_path
                        .extension()
                        .is_some_and(|ext| ext == "md")
                    {
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
                        }
                        else {
                            // Create destination url
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
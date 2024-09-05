use std::{
    env::temp_dir,
    fs::{create_dir_all, remove_dir_all, File},
    io::Write,
    panic,
};

use rand::{distributions::Alphanumeric, Rng};

use crate::{errors::SsgError, ConsoleArgs};

#[test]
fn site_with_links() -> anyhow::Result<()> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace"))
        .try_init();
    let temp_dir = temp_dir().join(temp_dir_name());
    let res = panic::catch_unwind(|| {
        {
            (|| {
                log::trace!("Creating nested directories");
                create_dir_all(&temp_dir.join("target/nested2"))?;
                create_dir_all(&temp_dir.join("target/nested3"))?;
                log::trace!("Done");
                let mut djot_file_1 = File::create(&temp_dir.join("target/index.dj"))?;
                write!(
                    djot_file_1,
                    "# Hey everyone!\n\nThis is an example djot file!\n\n> Hey what's up. Link:\n\n[HIHIDHI](nested2/hey.dj)"
                )?;
                log::trace!("Flushing file 1");
                djot_file_1.flush()?;
                log::trace!("Done");
                let mut djot_file_2 = File::create(&temp_dir.join("target/nested2/hey.dj"))?;
                write!(djot_file_2, "File 2\n\n### Hey\n\n[link](../index.dj)")?;
                log::trace!("Flushing file 2");
                djot_file_2.flush()?;
                log::trace!("Done");
                let mut djot_file_3 = File::create(&temp_dir.join("target/nested3/third_file.dj"))?;
                write!(
                    djot_file_3,
                    "File 3\n\n### What's good in the hous\n\n[link](../nested2/hey.dj)"
                )?;
                log::trace!("Flushing file 3");
                djot_file_3.flush()?;
                log::trace!("Djot files written");
                let args = ConsoleArgs {
                    target_path: temp_dir.join("target"),
                    output_path: Some(temp_dir.join("output")),
                    clean: false,
                    no_warn: true,
                    web_prefix: None,
                };
                log::trace!("Running program");
                crate::run_program(args)?;
                assert!(temp_dir.join("output/index.html").exists());
                assert!(!temp_dir.join("output/index.dj").exists());
                assert!(temp_dir.join("output/nested2/hey.html").exists());
                assert!(!temp_dir.join("output/nested2/hey.dj").exists());
                assert!(temp_dir.join("output/nested3/third_file.html").exists());
                assert!(!temp_dir.join("output/nested3/third_file.dj").exists());
                Ok(())
            })()
        }
    });

    log::trace!("Done with testing");

    let _ = remove_dir_all(&temp_dir);
    match res {
        Ok(e) => e,
        _ => Err(anyhow::anyhow!("Panic occurred")),
    }
}

#[test]
fn site_warn_without_index() -> anyhow::Result<()> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace"))
        .try_init();
    let temp_dir = temp_dir().join(temp_dir_name());
    // Perform test with catch
    let res = panic::catch_unwind(|| {
        {
            (|| -> anyhow::Result<()> {
                create_dir_all(&temp_dir.join("target/nested"))?;
                let mut djot_file_1 = File::create(&temp_dir.join("target/nested/example.dj"))?;
                write!(
                    djot_file_1,
                    "# Hey everyone!\n\nThis is an example djot file!\n\n> Hey what's up"
                )?;
                djot_file_1.flush()?;

                let mut djot_file_2 = File::create(&temp_dir.join("target/example2.dj"))?;
                write!(
                    djot_file_2,
                    "# Hey everyone!\n\nThis is another example djot file!\n\n> Hey what's up!!"
                )?;
                djot_file_2.flush()?;

                let args = ConsoleArgs {
                    target_path: temp_dir.join("target"),
                    output_path: Some(temp_dir.join("output")),
                    clean: false,
                    no_warn: true,
                    web_prefix: None,
                };
                crate::run_program(args)?;
                Ok(())
            })()
        }
    });

    let _ = remove_dir_all(&temp_dir);
    match res {
        Ok(e) => {
            match e {
                Ok(()) => {
                    // This should have errored out
                    Err(anyhow::anyhow!("This should have errored out"))
                }
                Err(e2) => match e2.downcast_ref::<SsgError>() {
                    Some(SsgError::IndexPageNotFound) => Ok(()),
                    _ => Err(e2),
                },
            }
        }
        _ => Err(anyhow::anyhow!("Panic occurred")),
    }
}

fn temp_dir_name() -> String {
    format!(
        ".simple-ssg-test-{}",
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect::<String>()
    )
}

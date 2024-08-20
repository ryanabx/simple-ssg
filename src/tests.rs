use std::{
    env::temp_dir,
    fs::{create_dir_all, remove_dir_all, File},
    io::Write,
};

use crate::ConsoleArgs;

#[test]
fn site_warn_without_index() -> anyhow::Result<()> {
    let temp_dir = temp_dir();
    // Perform test with catch
    let res: anyhow::Result<()> = (|| {
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
        };
        crate::run_program(args)?;
        Ok(())
    })();
    let _ = remove_dir_all(&temp_dir);
    match res {
        Ok(()) => {
            // This should have errored out
            Err(anyhow::anyhow!("This should have errored out"))
        }
        Err(e) => {
            // Ok(())
            if e.to_string() == "index.{dj|djot} not found! consider creating one in the base target directory as the default page.".to_string() {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

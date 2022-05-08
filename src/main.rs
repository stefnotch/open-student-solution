use fs_extra::copy_items;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Overengineered Student Solution Opener v1.0");

    // TODO: Check if directory is correct. Otherwise we do a little file browser hehe
    let algodat_dir = Path::new("../algodat-2022s-tutorinnen/ag1");

    for entry in fs::read_dir(algodat_dir)? {
        let dir = entry?;
        println!("{:?}", dir.path());
    }

    let args: Vec<String> = std::env::args().collect();
    let mut mat_nr = if args.len() > 1 {
        &(args[1])
    } else {
        "11740473"
    };
    if !is_mat_nr(mat_nr) {
        //mat_nr = "x";
        println!("Please pass a valid matriculation number as the first argument");
        return Ok(());
    }
    // TODO: Check for known matnr and ask user for one if it's not known

    // Don't use temp dir, since it's majorly cursed on Windows
    let output_solutions_dir = algodat_dir.join("opened-solution");
    fs::create_dir_all(&output_solutions_dir)?;

    let output_dir = output_solutions_dir.join("ad-".to_owned() + mat_nr);
    if output_dir.exists() && !output_dir.read_dir()?.next().is_none() {
        println!("The {:?} directory already exists", &output_dir);
        return Ok(());
        // TODO: Handle this case
    } else {
        fs::create_dir_all(&output_dir)?;
    }

    // No chmod 0700, since we're on Windows and stuff

    println!("Created {:?}", &output_dir);
    let mut gitignore_file = File::create(output_dir.join(".gitignore"))?;
    gitignore_file.write_all(b"/*")?; // Basically gitignore this entire folder

    // Copy the Frameworks
    let mut framework_files = Vec::new();
    {
        let options = fs_extra::dir::CopyOptions::new();
        let mut from_paths = Vec::new();
        for entry in fs::read_dir(algodat_dir.join("frameworks"))? {
            let dir = entry?;
            from_paths.push(dir.path());
            framework_files.push(dir.file_name())
        }
        copy_items(&from_paths, &output_dir, &options)?;
    }

    // Copy the abgabe PDF
    if let Some(student_pdf_file) = find_student_file(algodat_dir.join("abgaben/Berichte"), mat_nr)?
    {
        fs::copy(
            &student_pdf_file,
            output_dir.join(
                student_pdf_file
                    .file_name()
                    .unwrap_or(&std::ffi::OsString::from("abgabe.pdf")),
            ),
        )?;
    } else {
        println!("Couldn't find the abgabe PDF for {:?}", mat_nr);
    }

    // TODO: Copy the abgabe files (P1, P2, P3)
    {
        for entry in fs::read_dir(algodat_dir.join("abgaben"))? {
            let dir = entry?;
            if !framework_files.contains(&dir.file_name()) {
                continue;
            }

            if let Some(student_code_file) = find_student_file(dir.path(), mat_nr)? {
                println!(
                    "Found code {:?} {:?}",
                    dir.file_name(),
                    student_code_file.file_name()
                );
                fs::copy(
                    &student_code_file,
                    output_dir
                        .join(dir.file_name())
                        .join("src/main/java/exercise/StudentSolutionImplementation.java"),
                )?;
            } else {
                println!(
                    "Couldn't find the code file for {:?} {:?}",
                    dir.file_name(),
                    mat_nr
                );
            }
        }
    }
    // TODO: Show "open PDF" and "open P1/P2/P3" buttons
    // TODO: Protip, use https://plugins.jetbrains.com/plugin/14494-pdf-viewer and then you only have to share your screen
    // TODO: Open PDF with default program (or browser) and P1/P2/P3 with explorer

    // TODO: intellij (uh oh) with https://github.com/oliverschwendener/ueli/blob/dev/src/main/executors/application-searcher.ts or https://github.com/microsoft/windows-rs with https://stackoverflow.com/questions/908850/get-installed-applications-in-a-system

    Ok(())
}

fn find_student_file<P>(path: P, mat_nr: &str) -> std::io::Result<Option<PathBuf>>
where
    P: AsRef<Path>,
{
    for entry in fs::read_dir(path)? {
        let dir = entry?;
        if dir.file_name().to_string_lossy().contains(mat_nr) {
            return Ok(Some(dir.path()));
        }
    }

    return Ok(None);
}

struct Student {
    mat_nr: String,
    first_name: String,
    last_name: String,
}

/*
fn get_known_students() -> Vec<Student> {
    WalkDir::new(algodat_dir.join("abgaben"))
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|v| v.ok())
        .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().contains(mat_nr))
}*/

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|v| v.starts_with("."))
        .unwrap_or(false)
}

fn is_mat_nr(s: &str) -> bool {
    // see https://wiki.fsinf.at/wiki/Matrikelnummer
    s.chars().all(|c| c.is_ascii_digit()) && s.len() == 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matnr() -> Result<(), String> {
        assert_eq!(is_mat_nr("2"), false);
        assert_eq!(is_mat_nr("02"), false);
        assert_eq!(is_mat_nr("25222522"), true);
        assert_eq!(is_mat_nr("01234567"), true);
        Ok(())
    }
}

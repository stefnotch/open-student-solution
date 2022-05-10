use arboard::Clipboard;
use fs_extra::copy_items;
use std::collections::HashSet;
use std::fmt;
use std::{
    fs::{self, File},
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Overengineered Student Solution Opener v1.0");
    println!("Please place this in the algodat-2022s-tutorinnen/ag1 folder");

    if cfg!(debug_assertions) {
        println!("Warning: Running in dev mode");
    }

    // TODO: Check if directory is correct. Otherwise we do a little file browser hehe
    let algodat_dir = if cfg!(debug_assertions) {
        Path::new("../algodat-2022s-tutorinnen/ag1")
    } else {
        Path::new(".")
    };

    let algodat_abgaben_dir = algodat_dir.join("abgaben");

    let students = get_known_students(algodat_dir);

    let args: Vec<String> = std::env::args().collect();
    let mut mat_nr = if args.len() > 1 {
        args[1].to_owned()
    } else {
        "".to_owned()
    };

    // Get the matriculation number from the clipboard, if possible
    if !is_mat_nr(&mat_nr) {
        match Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
            Ok(clipboard_text) => {
                if is_mat_nr(&clipboard_text) {
                    let message = format!("Select student: {}", clipboard_text);
                    match inquire::Select::new(&message, vec![OkCancel::Ok, OkCancel::Cancel])
                        .prompt_skippable()
                    {
                        Ok(Some(OkCancel::Ok)) => {
                            mat_nr = clipboard_text;
                        }
                        _ => (),
                    };
                }
            }
            Err(_) => {}
        };
    }

    // Ask for a matriculation number
    let suggester = student_suggester(&students);
    while !is_mat_nr(&mat_nr) {
        mat_nr = match inquire::Text::new("Matriculation number?")
            .with_suggester(suggester.as_ref())
            .prompt()
        {
            Ok(user_mat_nr) => user_mat_nr.trim().chars().take(8).collect(), // TODO: This is a biiit of a hack
            Err(_) => {
                println!("An error happened when asking for the mat.nr., try again later.");
                return Ok(());
            }
        }
    }

    // Don't use temp dir, since it's majorly cursed on Windows
    let output_container_dir = algodat_dir.join("opened-solution");
    fs::create_dir_all(&output_container_dir)?;

    let output_dir = output_container_dir.join("ad-".to_owned() + &mat_nr);
    if output_dir.exists() && !output_dir.read_dir()?.next().is_none() {
        println!("The {:?} directory already exists", &output_dir);
        let options = vec![FileExistsOptions::Overwrite, FileExistsOptions::Cancel];
        match inquire::Select::new("What do you want to do?", options).prompt() {
            Ok(option) => match option {
                FileExistsOptions::Overwrite => {
                    fs::remove_dir_all(&output_dir)?;
                }
                FileExistsOptions::Cancel => {
                    println!("Cancelled...quitting program");
                    return Ok(());
                }
            },
            Err(_) => {
                println!(
                    "An error happened while dealing with the existing directory, try again later."
                );
                return Ok(());
            }
        }
    }

    fs::create_dir_all(&output_dir)?;

    // No chmod 0700, since we're on Windows and stuff

    println!("Created {:?}", &output_dir);
    let mut gitignore_file = File::create(output_dir.join(".gitignore"))?;
    gitignore_file.write_all(b"/*")?; // Basically gitignore this entire folder

    // Copy the Frameworks
    let framework_dirs = get_framework_dirs(algodat_dir)?;
    let framework_files: Vec<_> = framework_dirs.iter().map(fs::DirEntry::file_name).collect();
    {
        let options = fs_extra::dir::CopyOptions::new();
        let from_paths: Vec<_> = framework_dirs.iter().map(fs::DirEntry::path).collect();
        copy_items(&from_paths, &output_dir, &options)?;
    }

    // Copy the abgabe PDF
    if let Some(student_pdf_file) =
        find_student_file(algodat_abgaben_dir.join("Berichte"), &mat_nr)?
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

    // Copy the abgabe files (P1, P2, P3)
    {
        for entry in fs::read_dir(algodat_abgaben_dir)? {
            let source_dir = entry?;
            if !framework_files.contains(&source_dir.file_name()) {
                continue;
            }
            let result_dir = output_dir.join(source_dir.file_name());

            if let Some(student_code_file) = find_student_file(source_dir.path(), &mat_nr)? {
                println!(
                    "Found code {:?} {:?}",
                    source_dir.file_name(),
                    student_code_file.file_name()
                );
                fs::copy(
                    &student_code_file,
                    result_dir.join("src/main/java/exercise/StudentSolutionImplementation.java"),
                )?;

                let mut named_student_code_file = source_dir.file_name();
                named_student_code_file.push("-StudentSolutionImplementation.java");
                fs::copy(&student_code_file, output_dir.join(named_student_code_file))?;
            } else {
                println!(
                    "Couldn't find the code file for {:?} {:?}",
                    source_dir.file_name(),
                    mat_nr
                );
                let mut empty_solution_name = source_dir.file_name();
                empty_solution_name.push("-empty");
                fs::rename(&result_dir, result_dir.with_file_name(empty_solution_name))?;
            }
        }
    }

    open::that(&output_dir)?;
    // TODO: Show "open PDF" and "open P1/P2/P3" buttons
    // TODO: Protip, use https://plugins.jetbrains.com/plugin/14494-pdf-viewer and then you only have to share your screen
    // TODO: Open PDF with default program (or browser) and P1/P2/P3 with explorer
    // TODO: https://tuwel.tuwien.ac.at/mod/assign/view.php?action=grading&id=1456555&tifirst=A&tilast=A
    // TODO: Ask "open with Intellij/default program"

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

fn get_framework_dirs(algodat_dir: &Path) -> std::io::Result<Vec<fs::DirEntry>> {
    let mut framework_dirs = Vec::new();
    for entry in fs::read_dir(algodat_dir.join("frameworks"))? {
        let dir = entry?;
        framework_dirs.push(dir);
    }
    return Ok(framework_dirs);
}

enum OkCancel {
    Ok,
    Cancel,
}

impl fmt::Display for OkCancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OkCancel::Ok => write!(f, "Ok"),
            OkCancel::Cancel => write!(f, "Cancel"),
        }
    }
}

enum FileExistsOptions {
    Overwrite,
    // TODO: Add a retry option
    // TODO: Add a "Don't delete (rename)" option
    Cancel,
}

impl fmt::Display for FileExistsOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileExistsOptions::Overwrite => write!(f, "Overwrite"),
            FileExistsOptions::Cancel => write!(f, "Cancel"),
        }
    }
}

struct Student {
    mat_nr: String,
    first_name: String,
    last_name: String,
}
impl PartialEq for Student {
    fn eq(&self, other: &Self) -> bool {
        self.mat_nr == other.mat_nr
    }
}

impl Eq for Student {}

impl Hash for Student {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mat_nr.hash(state)
    }
}

fn student_suggester<'a>(
    students: &'a Vec<Student>,
) -> Box<dyn for<'r> Fn(&'r str) -> Vec<String> + 'a> {
    Box::new(|user_input| {
        students
            .iter()
            .filter(|student| {
                student.mat_nr.starts_with(user_input)
                    || student.first_name.contains(user_input)
                    || student.last_name.contains(user_input)
                // TODO: firstname lastname search or fuzzy search
            })
            .map(|student| {
                format!(
                    "{} - {} {}",
                    student.mat_nr, student.first_name, student.last_name
                )
            })
            .collect()
    })
}

fn get_known_students<P>(algodat_abgaben_dir: P) -> Vec<Student>
where
    P: AsRef<Path>,
{
    let all_students: Vec<_> = WalkDir::new(algodat_abgaben_dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|v| v.ok())
        .filter_map(|e| parse_student_file(e.path()))
        .collect();

    let students_set: HashSet<Student> = HashSet::from_iter(all_students);
    students_set.into_iter().collect()
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|v| v.starts_with("."))
        .unwrap_or(false)
}

fn parse_student_file(path: &Path) -> Option<Student> {
    path.file_stem().and_then(|file_name| {
        let file_name = file_name.to_string_lossy();
        let parts: Vec<&str> = file_name.split("-").collect();
        if parts.len() != 3 {
            return None;
        } else {
            return Some(Student {
                mat_nr: parts[2].to_string(),
                first_name: parts[1].to_string(),
                last_name: parts[0].to_string(),
            });
        }
    })
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

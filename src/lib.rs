/*!
Use this library in your build.rs to create a single file with all the crate's source code.

That's useful for programming exercise sites that take a single source file.
*/

use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

use regex::Regex;

const LIBRS_FILENAME: &str = "src/lib.rs";
const RESERVED_LIBRS_MOD_NAME: &str = "_reserved_librs";

#[derive(Debug, Clone)]
pub struct Bundler<'a> {
    _header: &'a str,
    binrs_filename: &'a Path,
    bundle_filename: &'a Path,
    librs_filename: &'a Path,
    comment_re: Regex,
    warn_re: Regex,
    _crate_name: &'a str,
    _silent: bool,
}

/// Defines a regex to match a line of rust source.
/// Uses a shorthand where "  " = "\s+" and " " = "\s*"
fn source_line_regex<S: AsRef<str>>(source_regex: S) -> Regex {
    Regex::new(
        format!(
            "^{}(?://.*)?$",
            source_regex
                .as_ref()
                .replace("  ", r"\s+")
                .replace(' ', r"\s*")
        )
        .as_str(),
    )
    .unwrap()
}

impl<'a> Bundler<'a> {
    pub fn new(binrs_filename: &'a Path, bundle_filename: &'a Path) -> Bundler<'a> {
        Bundler {
            _header: "",
            binrs_filename,
            bundle_filename,
            librs_filename: Path::new(LIBRS_FILENAME),
            comment_re: source_line_regex(r" "),
            warn_re: source_line_regex(r" #!\[warn\(.*"),
            _crate_name: "",
            _silent: false,
        }
    }

    pub fn crate_name(&mut self, name: &'a str) {
        self._crate_name = name;
    }

    pub fn header(&mut self, header: &'a str) {
        self._header = header;
    }

    pub fn silent(&mut self, silent: bool) {
        self._silent = silent;
    }

    pub fn run(&mut self) {
        let mut o = File::create(&self.bundle_filename)
            .unwrap_or_else(|_| panic!("error creating {}", &self.bundle_filename.display()));
        self.binrs(&mut o).unwrap_or_else(|_| {
            panic!(
                "error creating bundle {} for {}",
                self.bundle_filename.display(),
                self.binrs_filename.display()
            )
        });
        if !self._silent {
            println!("rerun-if-changed={}", self.bundle_filename.display());
        }
    }

    /// From the file that has the main() function, expand first "extern
    /// crate <_crate_name>" into lib.rs contents, and smartly skips
    /// "use <_crate_name>::" lines.
    fn binrs(&mut self, mut o: &mut File) -> Result<(), io::Error> {
        let bin_fd = File::open(self.binrs_filename)?;
        let mut bin_reader = BufReader::new(&bin_fd);

        let linemacro_re = source_line_regex(r" # \[.*");
        let empty_re = source_line_regex(r" ");
        let extcrate_re = source_line_regex(format!(r" extern  crate  {} ; ", self._crate_name));
        let usecrate_re = source_line_regex(
            format!(r" use  {} :: (.*) ; ", String::from(self._crate_name)).as_str(),
        );

        let mut line = String::new();
        let mut first = true;
        let mut line_macros = Vec::<String>::new();
        while bin_reader.read_line(&mut line).unwrap() > 0 {
            if line.ends_with('\n') {
                line = line[..line.len() - 1].to_string();
            }
            if self.comment_re.is_match(&line) || self.warn_re.is_match(&line) {
            } else if linemacro_re.is_match(&line) || empty_re.is_match(&line) {
                line_macros.push(line.clone());
            } else if extcrate_re.is_match(&line) {
                line_macros.clear();
                if first {
                    self.librs(o)?;
                    first = false;
                }
            } else if let Some(cap) = usecrate_re.captures(&line) {
                let moduse = cap.get(1).unwrap().as_str();
                if first {
                    self.librs(o)?;
                    first = false;
                }
                writeln!(&mut o, "use {}::{};", RESERVED_LIBRS_MOD_NAME, moduse)?;
            } else {
                if let Some(result) = line_macros
                    .iter()
                    .map(|e| self.write_line(o, e))
                    .find(|e| e.is_err())
                {
                    result?;
                }
                line_macros.clear();
                self.write_line(o, &line)?;
            }
            line.clear();
        }
        if let Some(result) = line_macros
            .iter()
            .map(|e| self.write_line(o, e))
            .find(|e| e.is_err())
        {
            result?;
        }
        Ok(())
    }

    /// Expand lib.rs contents and "pub mod <>;" lines.
    fn librs(&mut self, o: &mut File) -> Result<(), io::Error> {
        let lib_fd = File::open(self.librs_filename).expect("could not open lib.rs");
        let mut lib_reader = BufReader::new(&lib_fd);

        let mod_re = source_line_regex(r" (pub  )?mod  (?P<m>.+) ; ");

        let mut line = String::new();
        writeln!(o, "pub mod {} {{", RESERVED_LIBRS_MOD_NAME)?;
        while lib_reader.read_line(&mut line).unwrap() > 0 {
            line.pop();
            if self.comment_re.is_match(&line) || self.warn_re.is_match(&line) {
            } else if let Some(cap) = mod_re.captures(&line) {
                let modname = cap.name("m").unwrap().as_str();
                if modname != "tests" {
                    self.usemod(o, modname, modname, modname)?;
                }
            } else {
                self.write_line(o, &line)?;
            }
            line.clear(); // clear to reuse the buffer
        }
        writeln!(o, "}}")?;
        Ok(())
    }

    /// Called to expand random .rs files from lib.rs. It recursivelly
    /// expands further "pub mod <>;" lines and updates the list of
    /// "use <>;" lines that have to be skipped.
    fn usemod(
        &mut self,
        mut o: &mut File,
        mod_name: &str,
        mod_path: &str,
        mod_import: &str,
    ) -> Result<(), io::Error> {
        let mod_filenames0 = vec![
            format!("src/{}.rs", mod_path),
            format!("src/{}/mod.rs", mod_path),
        ];
        let mod_fd = mod_filenames0
            .iter()
            .map(|fn0| {
                let mod_filename = Path::new(&fn0);
                File::open(mod_filename)
            })
            .find(|fd| fd.is_ok());
        assert!(mod_fd.is_some(), "could not find file for module");
        let mut mod_reader = BufReader::new(mod_fd.unwrap().unwrap());

        let mod_re = source_line_regex(r" (pub  )?mod  (?P<m>.+) ; ");

        let mut line = String::new();

        writeln!(&mut o, "pub mod {} {{", mod_name)?;

        while mod_reader.read_line(&mut line).unwrap() > 0 {
            line.truncate(line.trim_end().len());
            if self.comment_re.is_match(&line) || self.warn_re.is_match(&line) {
            } else if let Some(cap) = mod_re.captures(&line) {
                let submodname = cap.name("m").unwrap().as_str();
                if submodname != "tests" {
                    let submodfile = format!("{}/{}", mod_path, submodname);
                    let submodimport = format!("{}::{}", mod_import, submodname);
                    self.usemod(o, submodname, submodfile.as_str(), submodimport.as_str())?;
                }
            } else {
                self.write_line(o, &line)?;
            }
            line.clear(); // clear to reuse the buffer
        }

        writeln!(&mut o, "}}")?;

        Ok(())
    }

    fn write_line(&self, mut o: &mut File, line: &str) -> Result<(), io::Error> {
        writeln!(&mut o, "{}", line)
    }
}

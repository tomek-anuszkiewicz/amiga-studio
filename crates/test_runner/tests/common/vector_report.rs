use super::schema::SingleStepTest;
use serde_json::{json, Value};
use std::cell::Cell;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

thread_local! {
    static GROUP: Cell<Option<&'static str>> = const { Cell::new(None) };
}

fn directory() -> Option<PathBuf> {
    std::env::var_os("SINGLESTEP_CASE_REPORT_DIR").map(PathBuf::from)
}

fn write_event(file: &mut File, value: &Value) -> io::Result<()> {
    // Append each complete record with one write and no user-space buffer.
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    file.write_all(&line)
}

fn open(path: PathBuf) -> io::Result<File> {
    File::options().create_new(true).write(true).open(path)
}

fn report_error(error: io::Error) {
    eprintln!("[ERROR] SingleStep case evidence is incomplete: {error}");
    if let Some(directory) = directory() {
        let _ = fs::write(
            directory.join("incomplete"),
            b"SingleStep case evidence is incomplete\n",
        );
    }
}

pub(super) fn select_groups<'a>(groups: impl Iterator<Item = &'a str>) {
    let Some(directory) = directory() else {
        return;
    };
    let write = || -> io::Result<()> {
        let mut index = open(directory.join("groups.md"))?;
        writeln!(index, "# SingleStep groups\n")?;
        writeln!(
            index,
            "The native event file shows the selected vectors, individual starts and outcomes. \
             Each completed outcome includes host seconds. A missing finish indicates an \
             unfinished vector. An absent group file means no vector selection was recorded.\n"
        )?;
        for name in groups {
            writeln!(
                index,
                "- [{name}]({name}.md) ([native events]({name}.jsonl))"
            )?;
        }
        Ok(())
    };
    if let Err(error) = write() {
        report_error(error);
    }
}

pub(super) fn set_group(name: Option<&'static str>) {
    GROUP.set(name);
}

pub(super) struct VectorReport {
    file: File,
    markdown: File,
}

impl VectorReport {
    pub(super) fn reserve<'a>(
        vector_file: &str,
        selection: &str,
        limit: Option<usize>,
        tests: impl Iterator<Item = &'a SingleStepTest>,
    ) -> Option<Self> {
        let directory = directory()?;
        let group = GROUP.get()?;
        let result = (|| {
            let mut file = open(directory.join(format!("{group}.jsonl")))?;
            let mut markdown = open(directory.join(format!("{group}.md")))?;
            writeln!(markdown, "# {group}\n")?;
            writeln!(
                markdown,
                "Selection `{selection}` from `{vector_file}`. [Selected vectors and starts]({group}.jsonl).\n"
            )?;
            let cases: Vec<_> = tests.map(|test| &test.name).collect();
            write_event(
                &mut file,
                &json!({"event": "selection", "group": group, "vector_file": vector_file,
                        "selection": selection, "sample_limit": limit, "vectors": cases,
                        "elapsed_unit": "seconds"}),
            )?;
            Ok(Self { file, markdown })
        })();
        match result {
            Ok(report) => Some(report),
            Err(error) => {
                report_error(error);
                None
            }
        }
    }

    fn append(report: &mut Option<Self>, record: &Value) {
        if let Some(active) = report {
            if let Err(error) = write_event(&mut active.file, record) {
                report_error(error);
                *report = None;
            }
        }
    }

    pub(super) fn start(report: &mut Option<Self>, test: &SingleStepTest, index: usize) {
        if report.is_some() {
            Self::append(
                report,
                &json!({"event": "case_started", "id": test.name, "vector_index": index, "attempt": 1}),
            );
        }
    }

    pub(super) fn finish(
        report: &mut Option<Self>,
        test: &SingleStepTest,
        index: usize,
        elapsed: Duration,
        diagnostic: Option<&str>,
    ) {
        if report.is_none() {
            return;
        }
        let outcome = if diagnostic.is_some() {
            "failed"
        } else {
            "passed"
        };
        Self::append(
            report,
            &json!({"event": "case_finished", "id": test.name, "vector_index": index, "attempt": 1,
                    "outcome": outcome, "seconds": elapsed.as_secs_f64(), "expected_cpu_clocks": test.length,
                    "diagnostic": diagnostic, "stack": null}),
        );
        if let Some(active) = report {
            let name = test.name.replace('`', "\\`");
            let mut text = format!(
                "- Vector {index} `{name}`. {outcome} in {:.9} seconds. Expected guest clocks {}.\n",
                elapsed.as_secs_f64(),
                test.length,
            );
            if let Some(diagnostic) = diagnostic {
                text.push_str("  Stack unavailable. See the native diagnostic for this vector.\n");
                // Indented continuation keeps diagnostic text independent of Markdown syntax.
                for line in diagnostic.lines() {
                    text.push_str("    ");
                    text.push_str(line);
                    text.push('\n');
                }
            }
            if let Err(error) = active.markdown.write_all(text.as_bytes()) {
                report_error(error);
            }
        }
    }
}

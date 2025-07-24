/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::borrow::Cow;
use std::cmp;
use std::fmt::Debug;
use std::io;
use std::io::Write;

use itertools::Itertools;
use pyrefly_python::module::Module;
use pyrefly_python::module_path::ModulePath;
use pyrefly_util::display::number_thousands;
use pyrefly_util::lined_buffer::DisplayRange;
use pyrefly_util::lined_buffer::LineNumber;
use pyrefly_util::lined_buffer::LinedBuffer;
use ruff_annotate_snippets::Level;
use ruff_annotate_snippets::Message;
use ruff_annotate_snippets::Renderer;
use ruff_annotate_snippets::Snippet;
use ruff_text_size::Ranged;
use ruff_text_size::TextRange;
use starlark_map::small_map::SmallMap;
use vec1::Vec1;
use yansi::Paint;

use crate::config::error_kind::ErrorKind;
use crate::config::error_kind::Severity;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Error {
    module: Module,
    range: TextRange,
    display_range: DisplayRange,
    error_kind: ErrorKind,
    severity: Severity,
    /// First line of the error message
    msg_header: Box<str>,
    /// The rest of the error message after the first line.
    /// Note that this is formatted for pretty-printing, with two spaces at the beginning and after every newline.
    msg_details: Option<Box<str>>,
}

impl Ranged for Error {
    fn range(&self) -> TextRange {
        self.range
    }
}

impl Error {
    pub fn write_line(&self, mut f: impl Write, verbose: bool) -> io::Result<()> {
        if verbose && self.severity.is_enabled() {
            writeln!(
                f,
                "{} {} [{}]",
                self.severity.label(),
                self.msg_header,
                self.error_kind.to_name(),
            )?;
            let origin = self.lossy_origin();
            let snippet = self.get_source_snippet(&origin);
            let renderer = Renderer::plain();
            writeln!(f, "{}", renderer.render(snippet))?;
            if let Some(details) = &self.msg_details {
                writeln!(f, "{details}")?;
            }
        } else if self.severity.is_enabled() {
            writeln!(
                f,
                "{} {}:{}: {} [{}]",
                self.severity.label(),
                self.path(),
                self.display_range,
                self.msg_header,
                self.error_kind.to_name(),
            )?;
        }
        Ok(())
    }

    pub fn print_colors(&self, verbose: bool) {
        if verbose && self.severity.is_enabled() {
            anstream::println!(
                "{} {} {}",
                self.severity.painted(),
                Paint::new(&*self.msg_header),
                Paint::dim(format!("[{}]", self.error_kind().to_name()).as_str()),
            );
            let origin = self.lossy_origin();
            let snippet = self.get_source_snippet(&origin);
            let renderer = Renderer::styled();
            anstream::println!("{}", renderer.render(snippet));
            if let Some(details) = &self.msg_details {
                anstream::println!("{details}");
            }
        } else if self.severity.is_enabled() {
            anstream::println!(
                "{} {}:{}: {} {}",
                self.severity.painted(),
                Paint::blue(&self.path().as_path().display()),
                Paint::dim(self.display_range()),
                Paint::new(&*self.msg_header),
                Paint::dim(format!("[{}]", self.error_kind().to_name()).as_str()),
            );
        }
    }

    fn lossy_origin(&self) -> Cow<'_, str> {
        self.path().as_path().to_string_lossy()
    }

    fn get_source_snippet<'a>(&'a self, origin: &'a str) -> Message<'a> {
        // Maximum number of lines to print in the snippet.
        const MAX_LINES: u32 = 5;

        // Warning: The SourceRange is char indexed, while the snippet is byte indexed.
        //          Be careful in the conversion.
        let source = self.module.lined_buffer().content_in_line_range(
            self.display_range.start.line,
            cmp::min(
                LineNumber::from_zero_indexed(
                    self.display_range.start.line.to_zero_indexed() + MAX_LINES,
                ),
                self.display_range.end.line,
            ),
        );
        let line_start = self
            .module
            .lined_buffer()
            .line_start(self.display_range.start.line);

        let level = match self.severity {
            Severity::Error => Level::Error,
            Severity::Warn => Level::Warning,
            Severity::Info => Level::Info,
            Severity::Ignore => Level::None,
        };
        let span_start = (self.range.start() - line_start).to_usize();
        let span_end = cmp::min(span_start + self.range.len().to_usize(), source.len());
        Level::None.title("").snippet(
            Snippet::source(source)
                .line_start(self.display_range.start.line.get() as usize)
                .origin(origin)
                .annotation(level.span(span_start..span_end)),
        )
    }

    pub fn with_severity(&self, severity: Severity) -> Self {
        let mut res = self.clone();
        res.severity = severity;
        res
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }
}

#[cfg(test)]
pub fn print_errors(errors: &[Error]) {
    for err in errors {
        err.print_colors(true);
    }
}

fn count_error_kinds(errors: &[Error]) -> Vec<(ErrorKind, usize)> {
    let mut map = SmallMap::new();
    for err in errors {
        let kind = err.error_kind();
        *map.entry(kind).or_default() += 1;
    }
    let mut res = map.into_iter().collect::<Vec<_>>();
    res.sort_by_key(|x| x.1);
    res
}

pub fn print_error_counts(errors: &[Error], limit: usize) {
    let items = count_error_kinds(errors);
    let limit = if limit > 0 { limit } else { items.len() };
    for (error, count) in items.iter().rev().take(limit) {
        eprintln!(
            "{} instances of {}",
            number_thousands(*count),
            error.to_name()
        );
    }
}

impl Error {
    pub fn new(module: Module, range: TextRange, msg: Vec1<String>, error_kind: ErrorKind) -> Self {
        let display_range = module.display_range(range);
        let msg_has_details = msg.len() > 1;
        let mut msg = msg.into_iter();
        let msg_header = msg.next().unwrap().into_boxed_str();
        let msg_details = if msg_has_details {
            Some(msg.map(|s| format!("  {s}")).join("\n").into_boxed_str())
        } else {
            None
        };
        Self {
            module,
            range,
            display_range,
            error_kind,
            severity: error_kind.default_severity(),
            msg_header,
            msg_details,
        }
    }

    pub fn display_range(&self) -> &DisplayRange {
        &self.display_range
    }

    pub fn lined_buffer(&self) -> &LinedBuffer {
        self.module.lined_buffer()
    }

    pub fn path(&self) -> &ModulePath {
        self.module.path()
    }

    pub fn msg_header(&self) -> &str {
        &self.msg_header
    }

    pub fn msg_details(&self) -> Option<&str> {
        self.msg_details.as_deref()
    }

    pub fn msg(&self) -> String {
        if let Some(details) = &self.msg_details {
            format!("{}\n{}", self.msg_header, details)
        } else {
            (*self.msg_header).to_owned()
        }
    }

    pub fn is_ignored(&self, permissive_ignores: bool) -> bool {
        self.module.is_ignored(
            &self.display_range,
            self.error_kind.to_name(),
            permissive_ignores,
        )
    }

    pub fn error_kind(&self) -> ErrorKind {
        self.error_kind
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::Arc;

    use pyrefly_python::module_name::ModuleName;
    use ruff_text_size::TextSize;
    use vec1::vec1;

    use super::*;

    #[test]
    fn test_error_render() {
        let module_info = Module::new(
            ModuleName::from_str("test"),
            ModulePath::filesystem(PathBuf::from("test.py")),
            Arc::new("def f(x: int) -> str:\n    return x".to_owned()),
        );
        let error = Error::new(
            module_info,
            TextRange::new(TextSize::new(26), TextSize::new(34)),
            vec1!["bad return".to_owned()],
            ErrorKind::BadReturn,
        );
        let mut normal = Vec::new();
        error
            .write_line(&mut Cursor::new(&mut normal), false)
            .unwrap();
        let mut verbose = Vec::new();
        error
            .write_line(&mut Cursor::new(&mut verbose), true)
            .unwrap();

        assert_eq!(
            str::from_utf8(&normal).unwrap(),
            "ERROR test.py:2:5-13: bad return [bad-return]\n"
        );
        assert_eq!(
            str::from_utf8(&verbose).unwrap(),
            r#"ERROR bad return [bad-return]
 --> test.py:2:5
  |
2 |     return x
  |     ^^^^^^^^
  |
"#,
        );
    }

    #[test]
    fn test_error_too_long() {
        let contents = format!("Start\n{}\nEnd", "X\n".repeat(1000));

        let module_info = Module::new(
            ModuleName::from_str("test"),
            ModulePath::filesystem(PathBuf::from("test.py")),
            Arc::new(contents.clone()),
        );
        let error = Error::new(
            module_info,
            TextRange::new(TextSize::new(0), TextSize::new(contents.len() as u32)),
            vec1!["oops".to_owned()],
            ErrorKind::BadReturn,
        );
        let mut output = Vec::new();
        error
            .write_line(&mut Cursor::new(&mut output), true)
            .unwrap();

        assert_eq!(
            str::from_utf8(&output).unwrap(),
            r#"ERROR oops [bad-return]
 --> test.py:1:1
  |
1 | / Start
2 | | X
3 | | X
4 | | X
5 | | X
6 | | X
  | |__^
  |
"#,
        );
    }
}

//! A small XML writer that indents with tabs, like InDesign's IDML export.

use std::fmt::Write;

pub struct Xml {
    out: String,
    stack: Vec<String>,
    /// The current element's start tag is still open (`<x a="b"`).
    open: bool,
    /// The current element has text content, so its end tag is not indented.
    inline: bool,
}

pub fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xa;"),
            '\r' => out.push_str("&#xd;"),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

impl Default for Xml {
    fn default() -> Self {
        Self::new()
    }
}

impl Xml {
    pub fn new() -> Xml {
        Xml {
            out: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n"),
            stack: Vec::new(),
            open: false,
            inline: false,
        }
    }

    fn close_start(&mut self) {
        if self.open {
            self.out.push('>');
            self.open = false;
        }
    }

    fn indent(&mut self) {
        self.out.push('\n');
        for _ in 0..self.stack.len() {
            self.out.push('\t');
        }
    }

    /// A processing instruction at the current position.
    pub fn pi(&mut self, text: &str) -> &mut Self {
        self.close_start();
        if self.stack.is_empty() {
            let _ = writeln!(self.out, "<?{text}?>");
        } else {
            let _ = write!(self.out, "<?{text}?>");
        }
        self
    }

    pub fn start(&mut self, name: &str) -> &mut Self {
        self.close_start();
        if !self.stack.is_empty() && !self.inline {
            self.indent();
        }
        let _ = write!(self.out, "<{name}");
        self.stack.push(name.to_string());
        self.open = true;
        self.inline = false;
        self
    }

    pub fn attr(&mut self, name: &str, value: impl AsRef<str>) -> &mut Self {
        debug_assert!(self.open, "attribute outside a start tag");
        let _ = write!(self.out, " {name}=\"{}\"", escape_attr(value.as_ref()));
        self
    }

    /// Text content. Characters below U+0020 other than tab become `<?ACE n?>`.
    pub fn text(&mut self, s: &str) -> &mut Self {
        self.close_start();
        self.inline = true;
        for ch in s.chars() {
            match ch {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                '\t' | '\n' => self.out.push(ch),
                c if (c as u32) < 0x20 => {
                    let _ = write!(self.out, "<?ACE {}?>", c as u32);
                }
                c => self.out.push(c),
            }
        }
        self
    }

    pub fn end(&mut self) -> &mut Self {
        let name = self.stack.pop().expect("end without start");
        if self.open {
            self.out.push_str(" />");
            self.open = false;
        } else {
            if !self.inline {
                self.indent();
            }
            let _ = write!(self.out, "</{name}>");
        }
        self.inline = false;
        self
    }

    /// An element with only attributes.
    pub fn empty(&mut self, name: &str, attrs: &[(&str, String)]) -> &mut Self {
        self.start(name);
        for (k, v) in attrs {
            self.attr(k, v);
        }
        self.end()
    }

    pub fn finish(mut self) -> String {
        while !self.stack.is_empty() {
            self.end();
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_nested_elements() {
        let mut x = Xml::new();
        x.start("a").attr("k", "1 < 2").start("b").end();
        x.start("c").text("hi & bye").end().end();
        let s = x.finish();
        assert!(
            s.ends_with("<a k=\"1 &lt; 2\">\n\t<b />\n\t<c>hi &amp; bye</c>\n</a>"),
            "{s}"
        );
    }

    #[test]
    fn control_characters_become_processing_instructions() {
        let mut x = Xml::new();
        x.start("Content").text("a\u{18}b").end();
        assert!(x.finish().ends_with("<Content>a<?ACE 24?>b</Content>"));
    }
}

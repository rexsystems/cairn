/// Turns Mojang's log4j XML console output back into readable lines.
#[derive(Default)]
pub struct Log4jParser {
    buf: Option<String>,
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn cdata(s: &str, tag: &str) -> Option<String> {
    let open = format!("<log4j:{tag}>");
    let start = s.find(&open)? + open.len();
    let end = s[start..].find(&format!("</log4j:{tag}>"))? + start;
    let inner = s[start..end].trim();
    let inner = inner.strip_prefix("<![CDATA[").and_then(|x| x.strip_suffix("]]>")).unwrap_or(inner);
    Some(inner.to_string())
}

fn format_event(ev: &str) -> String {
    let head_end = ev.find('>').unwrap_or(ev.len());
    let head = &ev[..head_end];
    let level = attr(head, "level").unwrap_or("INFO");
    let thread = attr(head, "thread").unwrap_or("main");
    let time = attr(head, "timestamp")
        .and_then(|t| t.parse::<i64>().ok())
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|d| d.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
        .unwrap_or_default();
    let mut out = format!("[{time}] [{thread}/{level}]: {}", cdata(ev, "Message").unwrap_or_default());
    if let Some(t) = cdata(ev, "Throwable") {
        out.push('\n');
        out.push_str(&t);
    }
    out
}

impl Log4jParser {
    /// Feeds one raw line; returns the lines to display (zero while inside an XML event).
    pub fn feed(&mut self, line: &str) -> Vec<String> {
        let t = line.trim_start();
        if let Some(buf) = &mut self.buf {
            buf.push_str(line);
            buf.push('\n');
            if t.starts_with("</log4j:Event>") {
                let ev = self.buf.take().unwrap();
                return format_event(&ev).lines().map(String::from).collect();
            }
            return vec![];
        }
        if t.starts_with("<log4j:Event") {
            if t.contains("</log4j:Event>") {
                return format_event(t).lines().map(String::from).collect();
            }
            self.buf = Some(format!("{line}\n"));
            return vec![];
        }
        vec![line.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_events() {
        let mut p = Log4jParser::default();
        assert!(p.feed(r#"<log4j:Event logger="ezv" timestamp="1760090000000" level="WARN" thread="Render thread">"#).is_empty());
        assert!(p.feed("  <log4j:Message><![CDATA[Setting user: Steve]]></log4j:Message>").is_empty());
        let out = p.feed("</log4j:Event>");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with("[Render thread/WARN]: Setting user: Steve"));
        assert_eq!(p.feed("plain line"), vec!["plain line".to_string()]);
    }
    #[test]
    fn throwable_lines() {
        let mut p = Log4jParser::default();
        p.feed(r#"<log4j:Event logger="a" timestamp="1" level="ERROR" thread="main">"#);
        p.feed("<log4j:Message><![CDATA[Boom]]></log4j:Message>");
        p.feed("<log4j:Throwable><![CDATA[java.lang.RuntimeException: x");
        p.feed("\tat a.b(C.java:1)");
        p.feed("]]></log4j:Throwable>");
        let out = p.feed("</log4j:Event>");
        assert_eq!(out.len(), 3);
        assert!(out[1].starts_with("java.lang.RuntimeException"));
    }
}

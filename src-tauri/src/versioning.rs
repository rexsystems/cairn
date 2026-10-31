//! Version ordering and the two predicate syntaxes mods declare: Fabric/Quilt
//! (`>=0.15 <0.17`, `1.20.x`, `~1.21`) and Forge/NeoForge Maven ranges (`[47,)`).
//! Every matcher returns `None` when it cannot parse, so callers never report a
//! conflict they are unsure about.

use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, Clone)]
enum Part {
    Num(u64),
    Text(String),
}

fn parts(s: &str) -> Vec<Part> {
    s.split(['.', '_'])
        .filter(|p| !p.is_empty())
        .map(|p| match p.parse::<u64>() {
            Ok(n) => Part::Num(n),
            Err(_) => Part::Text(p.to_ascii_lowercase()),
        })
        .collect()
}

fn cmp_parts(a: &[Part], b: &[Part]) -> Ordering {
    let n = a.len().max(b.len());
    for i in 0..n {
        let o = match (a.get(i), b.get(i)) {
            (Some(Part::Num(x)), Some(Part::Num(y))) => x.cmp(y),
            (Some(Part::Text(x)), Some(Part::Text(y))) => x.cmp(y),
            (Some(Part::Num(_)), Some(Part::Text(_))) => Ordering::Greater,
            (Some(Part::Text(_)), Some(Part::Num(_))) => Ordering::Less,
            (Some(Part::Num(x)), None) => x.cmp(&0),
            (None, Some(Part::Num(y))) => 0.cmp(y),
            (Some(Part::Text(_)), None) => Ordering::Less,
            (None, Some(Part::Text(_))) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

/// Splits `1.2.3-beta.1+build` into core and pre-release; build metadata is ignored.
fn split(v: &str) -> (&str, Option<&str>) {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let v = v.split('+').next().unwrap_or(v);
    match v.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (v, None),
    }
}

/// Semver-like ordering that also copes with Minecraft and loader version strings.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (ac, ap) = split(a);
    let (bc, bp) = split(b);
    match cmp_parts(&parts(ac), &parts(bc)) {
        Ordering::Equal => match (ap, bp) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(x), Some(y)) => cmp_parts(&parts(x), &parts(y)),
        },
        o => o,
    }
}

fn looks_like_version(v: &str) -> bool {
    let (core, _) = split(v);
    core.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
}

/// Matches `1.20.x` / `1.20.*` style wildcards against a version.
fn wildcard_matches(pattern: &str, v: &str) -> Option<bool> {
    let (pc, _) = split(pattern);
    let (vc, _) = split(v);
    let pp: Vec<&str> = pc.split('.').collect();
    let vp: Vec<&str> = vc.split('.').collect();
    for (i, p) in pp.iter().enumerate() {
        if *p == "x" || *p == "X" || *p == "*" {
            return Some(true);
        }
        let got = vp.get(i).copied().unwrap_or("0");
        if p.parse::<u64>().ok()? != got.parse::<u64>().ok()? {
            return Some(false);
        }
    }
    Some(true)
}

fn bump(core: &str, index: usize) -> String {
    let mut nums: Vec<u64> = core.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    while nums.len() <= index {
        nums.push(0);
    }
    nums[index] += 1;
    nums.truncate(index + 1);
    nums.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".")
}

fn term_matches(term: &str, v: &str) -> Option<bool> {
    let term = term.trim();
    if term.is_empty() || term == "*" {
        return Some(true);
    }
    for op in [">=", "<=", ">", "<", "=", "~", "^"] {
        if let Some(rest) = term.strip_prefix(op) {
            let rest = rest.trim();
            if rest.contains(['x', 'X', '*']) && op == "=" {
                return wildcard_matches(rest, v);
            }
            if !looks_like_version(rest) {
                return None;
            }
            let o = compare(v, rest);
            return Some(match op {
                ">=" => o != Ordering::Less,
                "<=" => o != Ordering::Greater,
                ">" => o == Ordering::Greater,
                "<" => o == Ordering::Less,
                "=" => o == Ordering::Equal,
                "~" => {
                    // ~1.2.3 means >=1.2.3 <1.3; ~1 means the 1.x line.
                    let (core, _) = split(rest);
                    let upper = bump(core, if core.split('.').count() > 1 { 1 } else { 0 });
                    o != Ordering::Less && compare(v, &upper) == Ordering::Less
                }
                _ => {
                    let (core, _) = split(rest);
                    let first_nonzero = core.split('.').position(|p| p != "0").unwrap_or(0);
                    let upper = bump(core, first_nonzero);
                    o != Ordering::Less && compare(v, &upper) == Ordering::Less
                }
            });
        }
    }
    if term.contains(['x', 'X', '*']) {
        return wildcard_matches(term, v);
    }
    if !looks_like_version(term) {
        return None;
    }
    Some(compare(v, term) == Ordering::Equal)
}

/// One Fabric predicate string: space-separated terms that must all match.
pub fn fabric_matches(pred: &str, v: &str) -> Option<bool> {
    let mut all = true;
    for term in pred.split_whitespace() {
        all &= term_matches(term, v)?;
    }
    Some(all)
}

/// Fabric allows a list of predicates; any one matching is enough.
pub fn fabric_any(preds: &[String], v: &str) -> Option<bool> {
    if preds.is_empty() {
        return Some(true);
    }
    let mut unknown = false;
    for p in preds {
        match fabric_matches(p, v) {
            Some(true) => return Some(true),
            Some(false) => {}
            None => unknown = true,
        }
    }
    if unknown {
        None
    } else {
        Some(false)
    }
}

/// Maven version range as used in `mods.toml`: `[1.20,1.21)`, `[47,)`, `[1.20.1]`, unions
/// separated by commas between brackets. A bare version is a soft requirement and matches anything.
pub fn maven_matches(range: &str, v: &str) -> Option<bool> {
    let range = range.trim();
    if range.is_empty() || range == "*" {
        return Some(true);
    }
    if !range.starts_with(['[', '(']) {
        return Some(true);
    }
    let mut rest = range;
    let mut any_parsed = false;
    while !rest.is_empty() {
        rest = rest.trim_start_matches([',', ' ']);
        if rest.is_empty() {
            break;
        }
        let open = rest.chars().next()?;
        if open != '[' && open != '(' {
            return None;
        }
        let end = rest.find([']', ')'])?;
        let close = rest[end..].chars().next()?;
        let body = &rest[1..end];
        rest = &rest[end + 1..];
        any_parsed = true;
        let ok = match body.split_once(',') {
            None => open == '[' && close == ']' && compare(v, body.trim()) == Ordering::Equal,
            Some((lo, hi)) => {
                let lo = lo.trim();
                let hi = hi.trim();
                let lo_ok = lo.is_empty() || {
                    let o = compare(v, lo);
                    if open == '[' { o != Ordering::Less } else { o == Ordering::Greater }
                };
                let hi_ok = hi.is_empty() || {
                    let o = compare(v, hi);
                    if close == ']' { o != Ordering::Greater } else { o == Ordering::Less }
                };
                lo_ok && hi_ok
            }
        };
        if ok {
            return Some(true);
        }
    }
    if any_parsed {
        Some(false)
    } else {
        None
    }
}

/// Release Minecraft versions (`1.20.1`, `26.1`) compare cleanly; snapshots do not.
pub fn is_release_mc(v: &str) -> bool {
    !v.is_empty() && v.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        assert_eq!(compare("1.20.1", "1.20"), Ordering::Greater);
        assert_eq!(compare("1.20", "1.20.0"), Ordering::Equal);
        assert_eq!(compare("0.16.10", "0.16.9"), Ordering::Greater);
        assert_eq!(compare("1.0.0-beta.2", "1.0.0"), Ordering::Less);
        assert_eq!(compare("1.0.0-beta.10", "1.0.0-beta.2"), Ordering::Greater);
        assert_eq!(compare("1.0.0+mc1.20", "1.0.0+mc1.21"), Ordering::Equal);
        assert_eq!(compare("v2.1", "2.1"), Ordering::Equal);
        assert_eq!(compare("26.1", "1.21.11"), Ordering::Greater);
    }

    #[test]
    fn fabric_predicates() {
        assert_eq!(fabric_matches("*", "1.20.1"), Some(true));
        assert_eq!(fabric_matches(">=0.15.0", "0.16.9"), Some(true));
        assert_eq!(fabric_matches(">=0.17", "0.16.9"), Some(false));
        assert_eq!(fabric_matches(">=1.20 <1.21", "1.20.6"), Some(true));
        assert_eq!(fabric_matches(">=1.20 <1.21", "1.21"), Some(false));
        assert_eq!(fabric_matches("1.20.x", "1.20.4"), Some(true));
        assert_eq!(fabric_matches("1.20.x", "1.21.1"), Some(false));
        assert_eq!(fabric_matches("~1.21", "1.21.4"), Some(true));
        assert_eq!(fabric_matches("~1.21.1", "1.22"), Some(false));
        assert_eq!(fabric_matches("^0.92.0", "0.92.6"), Some(true));
        assert_eq!(fabric_matches("^0.92.0", "0.93.0"), Some(false));
        assert_eq!(fabric_matches("^2.1", "2.9"), Some(true));
        assert_eq!(fabric_matches("1.21.1", "1.21.1"), Some(true));
        assert_eq!(fabric_matches("1.21.1", "1.21"), Some(false));
        assert_eq!(fabric_matches(">=1.21-alpha.24.10.a", "1.21.1"), Some(true));
        assert_eq!(fabric_matches("something weird", "1.0"), None);
    }

    #[test]
    fn fabric_lists() {
        let preds = vec!["1.20.1".to_string(), "1.20.2".to_string()];
        assert_eq!(fabric_any(&preds, "1.20.2"), Some(true));
        assert_eq!(fabric_any(&preds, "1.20.4"), Some(false));
        assert_eq!(fabric_any(&[], "1.20.4"), Some(true));
    }

    #[test]
    fn maven_ranges() {
        assert_eq!(maven_matches("[1.20,1.21)", "1.20.1"), Some(true));
        assert_eq!(maven_matches("[1.20,1.21)", "1.21"), Some(false));
        assert_eq!(maven_matches("[47,)", "47.2.0"), Some(true));
        assert_eq!(maven_matches("[47,)", "46.0.1"), Some(false));
        assert_eq!(maven_matches("[1.20.1]", "1.20.1"), Some(true));
        assert_eq!(maven_matches("[1.20.1]", "1.20.2"), Some(false));
        assert_eq!(maven_matches("(,1.19]", "1.19"), Some(true));
        assert_eq!(maven_matches("(1.19,1.20]", "1.19"), Some(false));
        assert_eq!(maven_matches("[1.0,2.0),[3.0,)", "3.1"), Some(true));
        assert_eq!(maven_matches("[1.0,2.0),[3.0,)", "2.5"), Some(false));
        assert_eq!(maven_matches("1.0", "0.1"), Some(true));
        assert_eq!(maven_matches("*", "0.1"), Some(true));
        assert_eq!(maven_matches("[broken", "0.1"), None);
    }

    #[test]
    fn release_versions() {
        assert!(is_release_mc("1.20.1"));
        assert!(is_release_mc("26.1"));
        assert!(!is_release_mc("24w14a"));
        assert!(!is_release_mc("1.21-pre1"));
    }
}

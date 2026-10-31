//! Reads a game log or crash report and explains why the game stopped, with one-click
//! fixes where Cairn can apply one.

use crate::compat::{action, Fix, FixAction, Issue, Severity};
use crate::modmeta::JarMod;
use once_cell::sync::Lazy;
use regex::Regex;

/// What the diagnosis needs to know about the instance.
pub struct Ctx<'a> {
    pub mods: &'a [JarMod],
    pub memory_mb: u32,
    /// Largest heap that leaves the system room, from `perf::advise`.
    pub max_safe_mb: u32,
}

/// Mod ids loaders report that aren't separate jars.
const NOT_MODS: &[&str] = &["minecraft", "java", "forge", "neoforge", "fml", "fabricloader", "fabric-loader", "quilt_loader", "mixinextras", "mcp"];

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid pattern")
}

static CLASS_VERSION: Lazy<Regex> = Lazy::new(|| re(r"class file version (\d+)(?:\.\d+)?\), this version of the Java Runtime only recognizes class file versions up to (\d+)"));
static UNSUPPORTED_MAJOR: Lazy<Regex> = Lazy::new(|| re(r"Unsupported class file major version (\d+)"));
static FABRIC_MISSING: Lazy<Regex> = Lazy::new(|| {
    re(r"(?m)^\s*- (?:Mod|Install) .*?requires .*?(?:'(?P<dname>[^']+)' \((?P<did>[\w.\-]+)\)|of (?P<plain>[\w.\-]+)), which is missing")
});
static FABRIC_INCOMPATIBLE: Lazy<Regex> = Lazy::new(|| re(r"(?m)^\s*- Mod '(?P<name>[^']+)' \((?P<id>[\w.\-]+)\) \S+ is incompatible with .*?'(?P<other>[^']+)' \((?P<oid>[\w.\-]+)\)"));
static FORGE_MISSING: Lazy<Regex> = Lazy::new(|| re(r"Mod ID: '(?P<id>[\w.\-]+)', Requested by: '(?P<by>[\w.\-]+)'.*?Actual version: '\[MISSING\]'"));
static MIXIN_MOD: Lazy<Regex> = Lazy::new(|| re(r"(?i)mixin.*?(?:for mod|from mod) ([\w\-]+)"));
static SUSPECTED: Lazy<Regex> = Lazy::new(|| re(r"(?m)^\s+(?P<name>.+?) \((?P<id>[\w\-]+)\), Version: "));
static DRIVER_FRAME: Lazy<Regex> = Lazy::new(|| re(r"(?i)(atio6axx|atioglxx|amdxx\w*|ig\w*icd\d*|nvoglv\d+)\.dll"));

fn issue(key: &str, severity: Severity, title: &str, detail: String, files: Vec<String>, fixes: Vec<FixAction>) -> Issue {
    Issue { key: format!("crash-{key}"), severity, title: title.into(), detail, files, fixes }
}

/// The jar that defines `id`, if any.
fn file_of<'a>(mods: &'a [JarMod], id: &str) -> Option<&'a JarMod> {
    mods.iter().find(|m| m.info.as_ref().is_some_and(|i| i.ids.iter().any(|x| x == id)))
}

fn disable(mods: &[JarMod], id: &str) -> Option<(String, FixAction)> {
    let m = file_of(mods, id).filter(|m| m.enabled)?;
    let name = m.info.as_ref().map(|i| i.name.clone()).unwrap_or_else(|| id.into());
    Some((m.file.clone(), action(format!("Disable {name}"), Fix::DisableMod { file: m.file.clone() })))
}

/// Java needed for a class file version (52 = Java 8).
fn java_for(class_version: u32) -> u32 {
    class_version.saturating_sub(44)
}

fn round_up_512(mb: u32) -> u32 {
    mb.div_ceil(512) * 512
}

pub fn diagnose(text: &str, ctx: &Ctx) -> Vec<Issue> {
    let mut out = vec![];

    if text.contains("java.lang.OutOfMemoryError") {
        let more = round_up_512(ctx.memory_mb + ctx.memory_mb / 2).min(ctx.max_safe_mb);
        let fixes = if more > ctx.memory_mb { vec![action(format!("Use {} GB", more as f32 / 1024.0), Fix::SetMemory { mb: more })] } else { vec![] };
        let detail = if fixes.is_empty() {
            "The game used all the memory it was given, and this computer has none to spare. Remove some heavy mods or lower the render distance.".into()
        } else {
            format!("The game used all {} GB it was given.", ctx.memory_mb as f32 / 1024.0)
        };
        out.push(issue("memory", Severity::Error, "Minecraft ran out of memory", detail, vec![], fixes));
    }
    if ["Could not reserve enough space for", "Invalid maximum heap size", "There is insufficient memory for the Java Runtime"].iter().any(|p| text.contains(p)) {
        let less = (ctx.memory_mb / 2).max(2048);
        let fixes = if less < ctx.memory_mb { vec![action(format!("Use {} GB", less as f32 / 1024.0), Fix::SetMemory { mb: less })] } else { vec![] };
        out.push(issue("heap", Severity::Error, "Java couldn't get the memory it was asked for", "The memory setting is more than this computer could give it.".into(), vec![], fixes));
    }

    if let Some(c) = CLASS_VERSION.captures(text) {
        let need = java_for(c[1].parse().unwrap_or(0));
        let have = java_for(c[2].parse().unwrap_or(0));
        out.push(issue(
            "java-old",
            Severity::Error,
            "This needs a newer Java",
            format!("Something here needs Java {need}, but the game ran on Java {have}."),
            vec![],
            vec![action("Let Cairn pick Java", Fix::UseManagedJava)],
        ));
    } else if let Some(c) = UNSUPPORTED_MAJOR.captures(text) {
        let have = java_for(c[1].parse().unwrap_or(0));
        out.push(issue(
            "java-new",
            Severity::Error,
            "This Java is too new for this version",
            format!("The game ran on Java {have}, which this loader version can't read."),
            vec![],
            vec![action("Let Cairn pick Java", Fix::UseManagedJava)],
        ));
    }

    let mut missing: Vec<(String, String)> = vec![];
    for c in FABRIC_MISSING.captures_iter(text) {
        let id = c.name("did").or(c.name("plain")).map(|m| m.as_str().to_string()).unwrap_or_default();
        let name = c.name("dname").map(|m| m.as_str().to_string()).unwrap_or_else(|| id.clone());
        missing.push((id, name));
    }
    for c in FORGE_MISSING.captures_iter(text) {
        missing.push((c["id"].to_string(), c["id"].to_string()));
    }
    missing.retain(|(id, _)| !id.is_empty() && !NOT_MODS.contains(&id.as_str()));
    missing.sort();
    missing.dedup_by(|a, b| a.0 == b.0);
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|(_, n)| n.as_str()).collect();
        let fixes = missing
            .iter()
            .take(4)
            .map(|(id, name)| action(format!("Install {name}"), Fix::InstallMod { mod_id: id.clone(), project_id: None, source: None }))
            .collect();
        out.push(issue("missing", Severity::Error, "Mods are missing what they need", format!("Missing: {}.", names.join(", ")), vec![], fixes));
    }

    for c in FABRIC_INCOMPATIBLE.captures_iter(text) {
        let (files, fixes): (Vec<_>, Vec<_>) = [&c["id"], &c["oid"]].into_iter().filter_map(|id| disable(ctx.mods, id)).unzip();
        out.push(issue(
            &format!("incompatible-{}-{}", &c["id"], &c["oid"]),
            Severity::Error,
            "Two mods can't be used together",
            format!("{} doesn't work with {}. Keep one of them.", &c["name"], &c["other"]),
            files,
            fixes,
        ));
    }

    let mut blamed: Vec<String> = vec![];
    if let Some(start) = text.find("Suspected Mod") {
        let block: String = text[start..].lines().skip(1).take_while(|l| l.starts_with([' ', '\t'])).collect::<Vec<_>>().join("\n");
        blamed.extend(SUSPECTED.captures_iter(&block).map(|c| c["id"].to_string()));
    }
    blamed.extend(MIXIN_MOD.captures_iter(text).map(|c| c[1].to_string()));
    blamed.retain(|id| !NOT_MODS.contains(&id.as_str()));
    blamed.dedup();
    let blamed: Vec<(String, FixAction)> = blamed.iter().filter_map(|id| disable(ctx.mods, id)).take(3).collect();
    if !blamed.is_empty() {
        let (files, fixes): (Vec<_>, Vec<_>) = blamed.into_iter().unzip();
        out.push(issue(
            "mod",
            Severity::Error,
            "A mod caused the crash",
            "The crash points at these mods. They may not support this Minecraft or loader version; disable one and try again.".into(),
            files,
            fixes,
        ));
    }

    if text.contains("Found duplicate mods") || text.contains("DuplicateModsFoundException") || text.contains("Duplicate mod") {
        out.push(issue(
            "duplicate",
            Severity::Error,
            "A mod is installed twice",
            "Two jars contain the same mod. Check mods on the Content tab shows which ones.".into(),
            vec![],
            vec![],
        ));
    }

    let driver = text.contains("EXCEPTION_ACCESS_VIOLATION").then(|| DRIVER_FRAME.captures(text)).flatten();
    if let Some(d) = driver {
        out.push(issue(
            "driver",
            Severity::Error,
            "The graphics driver crashed",
            format!("The crash happened inside {}.dll. Update your graphics driver from the GPU maker's site.", &d[1]),
            vec![],
            vec![],
        ));
    } else if ["Pixel format not accelerated", "GLFW error 65542", "does not appear to support OpenGL"].iter().any(|p| text.contains(p)) {
        out.push(issue(
            "opengl",
            Severity::Error,
            "OpenGL isn't available",
            "The graphics driver didn't offer OpenGL. Install the driver from your GPU maker; on laptops, make sure Java runs on the dedicated GPU.".into(),
            vec![],
            vec![],
        ));
    }
    out
}

/// True when the log points at mod loading, where a fresh compatibility check helps.
pub fn about_mods(issues: &[Issue]) -> bool {
    issues.iter().any(|i| ["crash-missing", "crash-duplicate"].contains(&i.key.as_str()) || i.key.starts_with("crash-incompatible"))
}

/// A one-line summary for the session history.
pub fn summary(issues: &[Issue]) -> Option<String> {
    issues.first().map(|i| i.title.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modmeta::ModInfo;

    fn jar(file: &str, id: &str, name: &str) -> JarMod {
        JarMod {
            file: file.into(),
            enabled: true,
            info: Some(ModInfo { ids: vec![id.into()], name: name.into(), ..Default::default() }),
        }
    }

    fn ctx(mods: &[JarMod]) -> Ctx<'_> {
        Ctx { mods, memory_mb: 4096, max_safe_mb: 12288 }
    }

    fn keys(v: &[Issue]) -> Vec<&str> {
        v.iter().map(|i| i.key.as_str()).collect()
    }

    #[test]
    fn out_of_memory_offers_more() {
        let v = diagnose("Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space", &ctx(&[]));
        assert_eq!(keys(&v), ["crash-memory"]);
        assert_eq!(v[0].fixes[0].fix, Fix::SetMemory { mb: 6144 });
        let tight = Ctx { mods: &[], memory_mb: 4096, max_safe_mb: 4096 };
        assert!(diagnose("java.lang.OutOfMemoryError", &tight)[0].fixes.is_empty());
    }

    #[test]
    fn heap_reservation_offers_less() {
        let v = diagnose("Error occurred during initialization of VM\nCould not reserve enough space for 8388608KB object heap", &Ctx { mods: &[], memory_mb: 8192, max_safe_mb: 4096 });
        assert_eq!(v[0].fixes[0].fix, Fix::SetMemory { mb: 4096 });
    }

    #[test]
    fn old_java_is_named() {
        let log = "java.lang.UnsupportedClassVersionError: net/minecraft/client/main/Main has been compiled by a more recent version of the Java Runtime (class file version 65.0), this version of the Java Runtime only recognizes class file versions up to 52.0";
        let v = diagnose(log, &ctx(&[]));
        assert_eq!(v[0].detail, "Something here needs Java 21, but the game ran on Java 8.");
        assert_eq!(v[0].fixes[0].fix, Fix::UseManagedJava);
        assert_eq!(keys(&diagnose("Unsupported class file major version 65", &ctx(&[]))), ["crash-java-new"]);
    }

    #[test]
    fn fabric_missing_dependencies() {
        let log = "net.fabricmc.loader.impl.FormattedException: Mod resolution encountered an incompatible mod set!\n\
            A potential solution has been determined:\n\
            \t - Install fabric-api, any version.\n\
            Unmet dependency listing:\n\
            \t - Mod 'Sodium Extra' (sodium-extra) 0.5.1 requires version 0.5.x of mod 'Sodium' (sodium), which is missing!\n\
            \t - Mod 'Mod Menu' (modmenu) 9.0.0 requires any version of fabric-api, which is missing!\n\
            \t - Mod 'X' (x) 1.0 requires version 0.15 or later of 'Fabric Loader' (fabricloader), which is missing!\n";
        let v = diagnose(log, &ctx(&[]));
        assert_eq!(keys(&v), ["crash-missing"]);
        let ids: Vec<_> = v[0].fixes.iter().map(|f| match &f.fix { Fix::InstallMod { mod_id, .. } => mod_id.as_str(), _ => "" }).collect();
        assert_eq!(ids, ["fabric-api", "sodium"]);
        assert!(about_mods(&v));
    }

    #[test]
    fn forge_missing_dependencies() {
        let log = "Missing or unsupported mandatory dependencies:\n\tMod ID: 'geckolib', Requested by: 'alexsmobs', Expected range: '[4.4,)', Actual version: '[MISSING]'\n\tMod ID: 'minecraft', Requested by: 'x', Expected range: '[1.20]', Actual version: '1.21'";
        let v = diagnose(log, &ctx(&[]));
        assert_eq!(v[0].fixes.len(), 1);
        assert_eq!(v[0].fixes[0].label, "Install geckolib");
    }

    #[test]
    fn incompatible_mods_can_be_disabled() {
        let mods = [jar("optifine.jar", "optifabric", "OptiFabric"), jar("sodium.jar", "sodium", "Sodium")];
        let log = "\t - Mod 'Sodium' (sodium) 0.5.8 is incompatible with any version of mod 'OptiFabric' (optifabric), but a matching version is present: 1.14!";
        let v = diagnose(log, &ctx(&mods));
        assert_eq!(v[0].files, ["sodium.jar", "optifine.jar"]);
        assert_eq!(v[0].fixes[1].fix, Fix::DisableMod { file: "optifine.jar".into() });
    }

    #[test]
    fn crash_report_blames_suspected_and_mixin_mods() {
        let mods = [jar("create.jar", "create", "Create"), jar("emi.jar", "emi", "EMI")];
        let report = "---- Minecraft Crash Report ----\nDescription: Ticking entity\n\nA detailed walkthrough:\nSuspected Mods: \n\tCreate (create), Version: 0.5.1\n\t\tat ...\n\
            Stacktrace:\nCaused by: org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError: Mixin [emi.mixins.json:ScreenMixin] from mod emi failed\n";
        let v = diagnose(report, &ctx(&mods));
        assert_eq!(keys(&v), ["crash-mod"]);
        assert_eq!(v[0].files, ["create.jar", "emi.jar"]);
    }

    #[test]
    fn mods_that_are_not_installed_get_no_button() {
        let report = "Suspected Mod: \n\tMinecraft (minecraft), Version: 1.21\n";
        assert!(diagnose(report, &ctx(&[])).is_empty());
    }

    #[test]
    fn graphics_driver_crashes() {
        let log = "# EXCEPTION_ACCESS_VIOLATION (0xc0000005)\n# Problematic frame:\n# C  [atio6axx.dll+0x1a2b]";
        assert_eq!(keys(&diagnose(log, &ctx(&[]))), ["crash-driver"]);
        assert_eq!(keys(&diagnose("GLFW error 65542: WGL: The driver does not appear to support OpenGL", &ctx(&[]))), ["crash-opengl"]);
    }

    #[test]
    fn clean_logs_have_no_findings() {
        assert!(diagnose("[Render thread/INFO]: Stopping!", &ctx(&[])).is_empty());
        assert_eq!(summary(&[]), None);
    }
}

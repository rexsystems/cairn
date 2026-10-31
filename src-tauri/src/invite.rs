//! Invites to a multiplayer session: the Discord join secret and the copyable invite code.
//! Discord hands the secret only to people it lets join, so it is the one place the address goes.

use crate::instance::LOADERS;
use crate::share::{self, Item, Manifest};
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Discord limits join secrets to 128 characters.
pub const MAX_SECRET: usize = 128;
const VERSION: &str = "c1";
const FP_LEN: usize = 11;

/// A modpack the joiner can install to match the host.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackRef {
    /// "m" for Modrinth, "c" for CurseForge.
    pub source: String,
    pub project: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    /// host:port the game connects to.
    pub address: String,
    pub mc: String,
    pub loader: String,
    pub lv: Option<String>,
    /// Fingerprint of the host's shareable content; None when the instance has none.
    pub fp: Option<String>,
    pub pack: Option<PackRef>,
}

fn plain(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c))
}

/// Short hash of the content list, independent of order. Two instances with the same
/// fingerprint have the same Modrinth/CurseForge files at the same versions.
pub fn fingerprint(items: &[Item]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let mut lines: Vec<String> = items.iter().map(|i| format!("{}:{}:{}", i.s, i.p, i.v)).collect();
    lines.sort();
    lines.dedup();
    let digest = Sha256::digest(lines.join("\n").as_bytes());
    Some(URL_SAFE_NO_PAD.encode(&digest[..8]))
}

fn ipv4(host: &str) -> Option<std::net::Ipv4Addr> {
    host.parse().ok()
}

/// A DNS name or public-looking IPv4 with an optional port; playit addresses rely on SRV records
/// and have none. Loopback and anything a server list entry wouldn't contain is refused.
pub fn valid_address(addr: &str) -> bool {
    let (host, port) = match addr.rsplit_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (addr, None),
    };
    let port_ok = port.is_none_or(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u32>().is_ok_and(|n| (1..=65535).contains(&n)));
    if !port_ok || host.is_empty() || host.len() > 253 {
        return false;
    }
    if let Some(ip) = ipv4(host) {
        return !ip.is_loopback() && !ip.is_unspecified() && !ip.is_broadcast() && !ip.is_multicast();
    }
    if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".localhost") {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty() && label.len() <= 63 && !label.starts_with('-') && !label.ends_with('-') && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }) && host.bytes().any(|b| b.is_ascii_alphabetic() || b == b'.')
}

fn pack_field(p: &PackRef) -> String {
    format!("{}:{}:{}", p.source, p.project, p.version)
}

fn parse_pack(s: &str) -> Result<Option<PackRef>> {
    if s.is_empty() {
        return Ok(None);
    }
    let parts: Vec<&str> = s.split(':').collect();
    match parts.as_slice() {
        [src @ ("m" | "c"), project, version] if plain(project, 40) && plain(version, 40) => {
            if *src == "c" && !(project.bytes().all(|b| b.is_ascii_digit()) && version.bytes().all(|b| b.is_ascii_digit())) {
                bail!("The invite names an invalid CurseForge pack");
            }
            Ok(Some(PackRef { source: src.to_string(), project: project.to_string(), version: version.to_string() }))
        }
        _ => bail!("The invite names an invalid pack"),
    }
}

fn check(inv: &Invite) -> Result<()> {
    if !valid_address(&inv.address) {
        bail!("The invite has an invalid server address");
    }
    if !plain(&inv.mc, 32) {
        bail!("The invite has an invalid Minecraft version");
    }
    if !LOADERS.contains(&inv.loader.as_str()) {
        bail!("The invite uses an unknown loader");
    }
    if inv.lv.as_deref().is_some_and(|v| !plain(v, 40)) {
        bail!("The invite has an invalid loader version");
    }
    if inv.fp.as_deref().is_some_and(|f| f.len() != FP_LEN || !f.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')) {
        bail!("The invite has an invalid content fingerprint");
    }
    Ok(())
}

fn join_fields(inv: &Invite) -> String {
    [
        VERSION,
        &inv.address,
        &inv.mc,
        &inv.loader,
        inv.lv.as_deref().unwrap_or(""),
        inv.fp.as_deref().unwrap_or(""),
        &inv.pack.as_ref().map(pack_field).unwrap_or_default(),
    ]
    .join(";")
}

/// Over Discord's 128-character limit, the loader version and then the pack reference are dropped;
/// the joiner then uses the newest loader or asks for the invite code.
pub fn encode(inv: &Invite) -> Result<String> {
    check(inv)?;
    let mut inv = inv.clone();
    let mut s = join_fields(&inv);
    if s.len() > MAX_SECRET {
        inv.lv = None;
        s = join_fields(&inv);
    }
    if s.len() > MAX_SECRET {
        inv.pack = None;
        s = join_fields(&inv);
    }
    if s.len() > MAX_SECRET {
        bail!("The server address is too long for a Discord invite");
    }
    Ok(s)
}

/// Parses a join secret delivered by Discord. Anything off-format is refused.
pub fn parse(secret: &str) -> Result<Invite> {
    if secret.len() > MAX_SECRET || !secret.is_ascii() {
        bail!("The Discord invite is not one Cairn made");
    }
    let f: Vec<&str> = secret.split(';').collect();
    let [v, address, mc, loader, lv, fp, pack] = f.as_slice() else { bail!("The Discord invite is not one Cairn made") };
    if *v != VERSION {
        bail!("The Discord invite comes from a different Cairn version");
    }
    let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());
    let inv = Invite { address: address.to_string(), mc: mc.to_string(), loader: loader.to_string(), lv: opt(lv), fp: opt(fp), pack: parse_pack(pack)? };
    check(&inv)?;
    Ok(inv)
}

/// The copyable fallback for people without Discord: a share code with the address after '@'.
/// The share payload is base64url, which never contains '@'.
pub fn code(share_code: &str, address: &str) -> Result<String> {
    if !valid_address(address) {
        bail!("Pick a valid server address first");
    }
    Ok(format!("{share_code}@{address}"))
}

/// Splits a pasted share or invite code. The address, when present, is checked like a secret's.
pub fn parse_code(text: &str) -> Result<(Manifest, Option<String>)> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    match compact.rsplit_once('@') {
        Some((share_code, address)) => {
            if !valid_address(address) {
                bail!("The invite code has an invalid server address");
            }
            Ok((share::decode(share_code)?, Some(address.to_string())))
        }
        None => Ok((share::decode(&compact)?, None)),
    }
}

/// Builds the invite for a host from their instance's manifest.
pub fn for_instance(address: &str, m: &Manifest, pack: Option<&crate::instance::PackSource>) -> Result<Invite> {
    let pack = pack.map(|p| PackRef {
        source: if !p.project_id.is_empty() && p.project_id.bytes().all(|b| b.is_ascii_digit()) { "c" } else { "m" }.into(),
        project: p.project_id.clone(),
        version: p.version_id.clone(),
    });
    let inv = Invite { address: address.to_string(), mc: m.mc.clone(), loader: m.loader.clone(), lv: m.lv.clone(), fp: fingerprint(&m.items), pack };
    check(&inv).map_err(|e| anyhow!("{e}"))?;
    Ok(inv)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: String,
    pub name: String,
}

/// What the joiner has to do before connecting.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Step {
    /// A local instance already matches: launch it.
    Launch { instance: Candidate },
    /// Install the host's modpack, then launch.
    InstallPack { pack: PackRef },
    /// Rebuild the instance from the full manifest (invite codes carry it).
    Rebuild,
    /// No content to match: a plain instance of the same version and loader is enough.
    Create,
    /// The host has mods Discord's secret can't describe; the invite code is needed.
    NeedCode,
}

/// Local instance info the planner compares against.
pub struct Local {
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub lv: Option<String>,
    pub fp: Option<String>,
    pub last_played: i64,
}

/// Decides how to get the joiner into the session. `have_manifest` is true for invite codes.
pub fn plan(inv: &Invite, locals: &[Local], have_manifest: bool) -> Step {
    let fits = |l: &&Local| l.mc == inv.mc && l.loader == inv.loader && l.fp == inv.fp && (inv.lv.is_none() || l.lv == inv.lv);
    if let Some(l) = locals.iter().filter(fits).max_by_key(|l| l.last_played) {
        return Step::Launch { instance: Candidate { id: l.id.clone(), name: l.name.clone() } };
    }
    if have_manifest {
        return Step::Rebuild;
    }
    if let Some(p) = &inv.pack {
        return Step::InstallPack { pack: p.clone() };
    }
    if inv.fp.is_none() {
        return Step::Create;
    }
    Step::NeedCode
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Invite {
        Invite {
            address: "abc-def.gl.joinmc.link:25565".into(),
            mc: "1.21.1".into(),
            loader: "fabric".into(),
            lv: Some("0.16.9".into()),
            fp: fingerprint(&[Item { s: "m".into(), p: "AANobbMI".into(), v: "v1".into() }]),
            pack: Some(PackRef { source: "m".into(), project: "1KVo5zza".into(), version: "Wz2mWy7c".into() }),
        }
    }

    #[test]
    fn secret_round_trips_within_discord_limit() {
        let s = encode(&sample()).unwrap();
        assert!(s.len() <= MAX_SECRET);
        assert_eq!(parse(&s).unwrap(), sample());
    }

    #[test]
    fn long_secrets_drop_loader_version_then_pack() {
        let mut inv = sample();
        inv.address = format!("{}.example.com:25565", "a".repeat(60));
        inv.pack = Some(PackRef { source: "m".into(), project: "p".repeat(30), version: "v".repeat(30) });
        let s = encode(&inv).unwrap();
        assert!(s.len() <= MAX_SECRET);
        let back = parse(&s).unwrap();
        assert_eq!(back.lv, None);
        assert_eq!(back.pack, None);
        assert_eq!(back.address, inv.address);
        let mut huge = sample();
        huge.address = format!("{}.{}.example.com:25565", "a".repeat(63), "b".repeat(63));
        assert!(encode(&huge).is_err());
    }

    #[test]
    fn parser_refuses_anything_off_format() {
        let good = encode(&sample()).unwrap();
        for bad in [
            "".to_string(),
            "c1;host.example:25565;1.21.1;fabric".into(),
            good.replace("c1;", "c2;"),
            format!("{good};extra"),
            good.replace("fabric", "rift"),
            good.replace("1.21.1", "1.21/../x"),
            good.replace(":25565", ":0"),
            good.replace(":25565", ":70000"),
            good.replace(":25565", ":"),
            good.replace("abc-def.gl.joinmc.link", "127.0.0.1"),
            good.replace("abc-def.gl.joinmc.link", "localhost"),
            good.replace("abc-def.gl.joinmc.link", "0.0.0.0"),
            good.replace("abc-def.gl.joinmc.link", "evil host"),
            good.replace("abc-def.gl.joinmc.link", "-bad.example"),
            good.replace("m:1KVo5zza:Wz2mWy7c", "x:1:2"),
            good.replace("m:1KVo5zza:Wz2mWy7c", "c:abc:1"),
            good.replace("m:1KVo5zza:Wz2mWy7c", "m:../a:b"),
            format!("{good}\u{e9}"),
            "c1;".repeat(50),
        ] {
            assert!(parse(&bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn addresses_allow_lan_and_dns_but_not_loopback() {
        assert!(valid_address("192.168.1.20:25565"));
        assert!(valid_address("play.example.org:25565"));
        assert!(!valid_address("127.0.0.1:25565"));
        assert!(!valid_address("[::1]:25565"));
        assert!(valid_address("fun-cat.joinmc.link"));
        assert!(!valid_address("example.org:"));
        assert!(!valid_address("a..b:1"));
        assert!(!valid_address("123:25565"));
    }

    #[test]
    fn fingerprint_ignores_order_and_is_none_for_vanilla() {
        let a = Item { s: "m".into(), p: "a".into(), v: "1".into() };
        let b = Item { s: "c".into(), p: "2".into(), v: "3".into() };
        assert_eq!(fingerprint(&[a.clone(), b.clone()]), fingerprint(&[b, a]));
        assert_eq!(fingerprint(&[]), None);
        assert_eq!(fingerprint(&[Item { s: "m".into(), p: "a".into(), v: "1".into() }]).unwrap().len(), FP_LEN);
    }

    #[test]
    fn invite_codes_carry_the_address_after_the_share_code() {
        let m = Manifest { name: "Pack".into(), mc: "1.21.1".into(), loader: "fabric".into(), lv: None, items: vec![] };
        let c = code(&share::encode(&m).unwrap(), "192.168.1.20:25565").unwrap();
        let (back, addr) = parse_code(&format!(" {c}\n")).unwrap();
        assert_eq!(back, m);
        assert_eq!(addr.as_deref(), Some("192.168.1.20:25565"));
        assert_eq!(parse_code(&share::encode(&m).unwrap()).unwrap().1, None);
        assert!(parse_code(&format!("{}@127.0.0.1:25565", share::encode(&m).unwrap())).is_err());
        assert!(code("CAIRN1:x", "no pe").is_err());
    }

    fn local(id: &str, fp: Option<String>, played: i64) -> Local {
        Local { id: id.into(), name: id.into(), mc: "1.21.1".into(), loader: "fabric".into(), lv: Some("0.16.9".into()), fp, last_played: played }
    }

    #[test]
    fn plan_prefers_a_matching_instance_then_pack_then_create() {
        let inv = sample();
        let locals = vec![local("old", inv.fp.clone(), 1), local("new", inv.fp.clone(), 5), local("other", None, 9)];
        assert_eq!(plan(&inv, &locals, false), Step::Launch { instance: Candidate { id: "new".into(), name: "new".into() } });
        assert_eq!(plan(&inv, &[], false), Step::InstallPack { pack: inv.pack.clone().unwrap() });
        assert_eq!(plan(&inv, &[], true), Step::Rebuild);
        let mut no_pack = inv.clone();
        no_pack.pack = None;
        assert_eq!(plan(&no_pack, &[], false), Step::NeedCode);
        let vanilla = Invite { fp: None, pack: None, loader: "vanilla".into(), lv: None, ..inv.clone() };
        assert_eq!(plan(&vanilla, &locals, false), Step::Create);
        let mut other_lv = inv;
        other_lv.lv = Some("0.15.0".into());
        assert_eq!(plan(&other_lv, &locals, true), Step::Rebuild);
    }
}

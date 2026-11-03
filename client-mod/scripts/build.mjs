import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const cache = path.join(root, '.cache');
const output = path.resolve(root, '../src-tauri/client-mods');
fs.mkdirSync(cache, { recursive: true });
fs.mkdirSync(output, { recursive: true });
const target = process.argv[2] ?? 'all';
if (!['all', 'forge', 'mapped', 'modern', 'test'].includes(target)) throw new Error('Use all, forge, mapped, modern or test.');
const executable = (name, javaHome = process.env.CAIRN_MODERN_JAVA_HOME) => javaHome ? path.join(javaHome, 'bin', name + (process.platform === 'win32' ? '.exe' : '')) : name;
function run(command, args, options = {}) { execFileSync(command, args, { stdio: 'inherit', ...options }); }
function sources(dir) {
    return fs.readdirSync(dir, { withFileTypes: true }).flatMap(entry => entry.isDirectory() ? sources(path.join(dir, entry.name)) : entry.name.endsWith('.java') ? [path.join(dir, entry.name)] : []);
}
function download(url, destination, expected, algorithm = 'sha1') {
    if (fs.existsSync(destination) && (!expected || crypto.createHash(algorithm).update(fs.readFileSync(destination)).digest('hex') === expected)) return destination;
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    const temporary = destination + '.download';
    run('curl', ['--fail', '--location', '--silent', '--show-error', '--retry', '2', url, '-o', temporary]);
    if (expected && crypto.createHash(algorithm).update(fs.readFileSync(temporary)).digest('hex') !== expected) { fs.rmSync(temporary); throw new Error(`Checksum mismatch for ${url}`); }
    fs.renameSync(temporary, destination);
    return destination;
}
const common = sources(path.join(root, 'common/src/main/java'));
if (target === 'all' || target === 'forge') {
    const env = { ...process.env };
    if (process.env.CAIRN_LEGACY_JAVA_HOME) env.JAVA_HOME = process.env.CAIRN_LEGACY_JAVA_HOME;
    for (const version of ['1.8', '1.8.8', '1.8.9']) {
        const args = ['-p', path.join(root, 'forge'), `-PminecraftVersion=${version}`, 'build', '--console=plain'];
        if (process.env.CAIRN_JAVA_INSTALLATIONS) args.push(`-Porg.gradle.java.installations.paths=${process.env.CAIRN_JAVA_INSTALLATIONS}`);
        run(process.env.CAIRN_GRADLE ?? 'gradle', args, { env });
        fs.copyFileSync(path.join(root, `forge/build/libs/cairn-client-forge-${version}-0.1.0.jar`), path.join(output, `cairn-client-forge-${version}.jar`));
    }
}
if (target === 'all' || target === 'mapped') {
    const env = { ...process.env };
    if (process.env.CAIRN_LEGACY_JAVA_HOME) env.JAVA_HOME = process.env.CAIRN_LEGACY_JAVA_HOME;
    for (const version of ['1.20.1', '1.21.1']) {
        const args = ['-p', path.join(root, 'fabric-mapped'), `-PminecraftVersion=${version}`, 'build', '--console=plain'];
        if (process.env.CAIRN_JAVA_INSTALLATIONS) args.push(`-Porg.gradle.java.installations.paths=${process.env.CAIRN_JAVA_INSTALLATIONS}`);
        run(process.env.CAIRN_GRADLE ?? 'gradle', args, { env });
        fs.copyFileSync(path.join(root, `fabric-mapped/build/libs/cairn-client-fabric-${version}-0.1.0.jar`), path.join(output, `cairn-client-fabric-${version}.jar`));
    }
}
if (target === 'all' || target === 'modern') {
    const metaFile = download('https://piston-meta.mojang.com/v1/packages/5df1795e16eea19d75b923d66ecca9ce183353c5/26.3.json', path.join(cache, '26.3.json'), '5df1795e16eea19d75b923d66ecca9ce183353c5');
    const meta = JSON.parse(fs.readFileSync(metaFile));
    const mc = download(meta.downloads.client.url, path.join(cache, 'minecraft-26.3.jar'), meta.downloads.client.sha1);
    const libs = [];
    for (const lib of meta.libraries) {
        if (lib.name.includes('natives-') || !lib.downloads?.artifact) continue;
        const artifact = lib.downloads.artifact;
        libs.push(download(artifact.url, path.join(cache, 'libs', artifact.path), artifact.sha1));
    }
    libs.push(download('https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.jar', path.join(cache, 'fabric-loader.jar'), 'd0c5a082e07410aaff9de7ab196aa6b4236002870f9ba3ad5c1260fcce5110ad', 'sha256'));
    libs.push(download('https://maven.fabricmc.net/net/fabricmc/sponge-mixin/0.17.5+mixin.0.8.7/sponge-mixin-0.17.5+mixin.0.8.7.jar', path.join(cache, 'mixin.jar'), '03f0065023bfc3045d71bfb99872451d9ad5a0800a5a0dac34ab8c7d5c134f5d', 'sha256'));
    const classes = path.join(root, 'modern/build/classes');
    fs.rmSync(classes, { recursive: true, force: true });
    fs.mkdirSync(classes, { recursive: true });
    run(executable('javac'), ['--release', '25', '-proc:none', '-cp', [mc, ...libs].join(path.delimiter), '-d', classes, ...common, ...sources(path.join(root, 'modern/src/main/java'))]);
    run(executable('jar'), ['--create', '--date=2026-01-01T00:00:00Z', '--file', path.join(output, 'cairn-client-fabric-26.3.jar'), '-C', classes, '.', '-C', path.join(root, 'modern/src/main/resources'), '.']);
}
if (target === 'all' || target === 'test') {
    const classes = path.join(root, 'common/build/test-classes');
    fs.mkdirSync(classes, { recursive: true });
    run(executable('javac'), ['--release', '8', '-d', classes, ...common, ...sources(path.join(root, 'tests'))]);
    run(executable('java'), ['-cp', classes, 'ClientCoreTest']);
    run(executable('java'), ['-cp', classes, 'app.cairn.client.ClientInteractionTest']);
}
console.log(`Cairn Client ${target}: complete.`);

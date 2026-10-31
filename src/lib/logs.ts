/** CSS class for a game log line: launcher notes, errors and stack frames, warnings. */
export function classify(line: string, stream: string): "launcher" | "error" | "warn" | "" {
  if (line.startsWith("[Cairn]")) return "launcher";
  if (/\/(ERROR|FATAL)\]|Exception|^\s+at /.test(line) || stream === "stderr" && /error/i.test(line)) return "error";
  if (/\/WARN\]/.test(line)) return "warn";
  return "";
}

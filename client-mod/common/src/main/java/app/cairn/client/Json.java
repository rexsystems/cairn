package app.cairn.client;

import java.util.*;

/** Tiny JSON reader; the shared core compiles without Minecraft's libraries, so it can't lean on Gson. */
final class Json {
    private final String text;
    private int at;
    private Json(String text) { this.text = text; }

    static Object parse(String text) {
        Json json = new Json(text);
        Object value = json.value(0);
        json.space();
        if (json.at != text.length()) throw new IllegalArgumentException("Trailing data at " + json.at);
        return value;
    }
    @SuppressWarnings("unchecked")
    static Map<String, Object> object(Object value) { return value instanceof Map ? (Map<String, Object>) value : Collections.<String, Object>emptyMap(); }
    @SuppressWarnings("unchecked")
    static List<Object> list(Object value) { return value instanceof List ? (List<Object>) value : Collections.emptyList(); }
    static String string(Object value, String fallback) { return value instanceof String ? (String) value : fallback; }
    static int integer(Object value, int fallback) { return value instanceof Number ? ((Number) value).intValue() : fallback; }
    static long whole(Object value, long fallback) { return value instanceof Number ? ((Number) value).longValue() : fallback; }

    static String quote(String value) {
        StringBuilder out = new StringBuilder(value.length() + 2).append('"');
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            if (c == '"' || c == '\\') out.append('\\').append(c);
            else if (c < 0x20) out.append(String.format(Locale.ROOT, "\\u%04x", (int) c));
            else out.append(c);
        }
        return out.append('"').toString();
    }

    private void space() { while (at < text.length() && Character.isWhitespace(text.charAt(at))) at++; }
    private IllegalArgumentException error(String what) { return new IllegalArgumentException(what + " at " + at); }
    private Object value(int depth) {
        // Server replies are shallow; the cap stops a hostile server from overflowing the stack.
        if (depth > 32) throw error("Too deep");
        space();
        if (at >= text.length()) throw error("Unexpected end");
        char c = text.charAt(at);
        if (c == '{') {
            at++;
            Map<String, Object> map = new LinkedHashMap<String, Object>();
            space();
            if (peek('}')) return map;
            do {
                space();
                String key = string();
                space();
                expect(':');
                map.put(key, value(depth + 1));
                space();
            } while (peek(','));
            expect('}');
            return map;
        }
        if (c == '[') {
            at++;
            List<Object> list = new ArrayList<Object>();
            space();
            if (peek(']')) return list;
            do { list.add(value(depth + 1)); space(); } while (peek(','));
            expect(']');
            return list;
        }
        if (c == '"') return string();
        if (text.startsWith("true", at)) { at += 4; return Boolean.TRUE; }
        if (text.startsWith("false", at)) { at += 5; return Boolean.FALSE; }
        if (text.startsWith("null", at)) { at += 4; return null; }
        int start = at;
        while (at < text.length() && "+-0123456789.eE".indexOf(text.charAt(at)) >= 0) at++;
        if (start == at) throw error("Unexpected '" + c + "'");
        String number = text.substring(start, at);
        try {
            if (number.indexOf('.') < 0 && number.indexOf('e') < 0 && number.indexOf('E') < 0) return Long.parseLong(number);
            return Double.parseDouble(number);
        } catch (NumberFormatException e) { throw error("Bad number"); }
    }
    private boolean peek(char c) { if (at < text.length() && text.charAt(at) == c) { at++; return true; } return false; }
    private void expect(char c) { if (!peek(c)) throw error("Expected '" + c + "'"); }
    private String string() {
        expect('"');
        StringBuilder out = new StringBuilder();
        while (true) {
            if (at >= text.length()) throw error("Unterminated string");
            char c = text.charAt(at++);
            if (c == '"') return out.toString();
            if (c != '\\') { out.append(c); continue; }
            if (at >= text.length()) throw error("Bad escape");
            char e = text.charAt(at++);
            switch (e) {
                case 'n': out.append('\n'); break;
                case 't': out.append('\t'); break;
                case 'r': out.append('\r'); break;
                case 'b': out.append('\b'); break;
                case 'f': out.append('\f'); break;
                case 'u':
                    if (at + 4 > text.length()) throw error("Bad escape");
                    try { out.append((char) Integer.parseInt(text.substring(at, at + 4), 16)); } catch (NumberFormatException x) { throw error("Bad escape"); }
                    at += 4; break;
                default: out.append(e);
            }
        }
    }
}

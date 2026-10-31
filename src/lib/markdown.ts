import { marked } from "marked";
import DOMPurify from "dompurify";

// Project bodies come from Modrinth authors, so treat them as hostile HTML.
const PURIFY = {
  FORBID_TAGS: ["style", "form", "input", "button", "textarea", "select", "iframe", "object", "embed"],
  FORBID_ATTR: ["style"],
};

export function renderProjectBody(markdown: string | null | undefined): string {
  if (!markdown) return "";
  return DOMPurify.sanitize(marked.parse(markdown, { async: false }) as string, PURIFY);
}

/** Only web links leave the launcher; anything else (file:, javascript:, custom schemes) is dropped. */
export function externalHref(href: string | null | undefined): string | null {
  if (!href) return null;
  try {
    const u = new URL(href);
    return u.protocol === "https:" || u.protocol === "http:" ? u.href : null;
  } catch {
    return null;
  }
}

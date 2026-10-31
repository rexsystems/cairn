// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { externalHref, renderProjectBody } from "./markdown";

describe("renderProjectBody", () => {
  it("renders markdown", () => {
    expect(renderProjectBody("**hi**")).toContain("<strong>hi</strong>");
  });
  it("strips scripts, handlers and javascript links", () => {
    const html = renderProjectBody('<img src="x" onerror="alert(1)"><script>alert(2)</script>[x](javascript:alert(3))');
    expect(html).not.toMatch(/onerror|<script|javascript:/i);
  });
  it("drops forms, iframes and inline styles", () => {
    const html = renderProjectBody('<form><input></form><iframe src="https://e.x"></iframe><p style="position:fixed">t</p>');
    expect(html).not.toMatch(/<form|<input|<iframe|style=/i);
    expect(html).toContain("<p>t</p>");
  });
  it("returns empty string for empty bodies", () => {
    expect(renderProjectBody(undefined)).toBe("");
  });
});

describe("externalHref", () => {
  it("allows web links only", () => {
    expect(externalHref("https://modrinth.com/mod/sodium")).toBe("https://modrinth.com/mod/sodium");
    expect(externalHref("file:///etc/passwd")).toBeNull();
    expect(externalHref("javascript:alert(1)")).toBeNull();
    expect(externalHref("not a url")).toBeNull();
  });
});

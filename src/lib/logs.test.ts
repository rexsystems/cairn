import { describe, expect, it } from "vitest";
import { classify } from "./logs";

describe("classify", () => {
  it("marks launcher lines", () => {
    expect(classify("[Cairn] Launching 1.21.1", "stdout")).toBe("launcher");
  });
  it("marks errors, exceptions and stack frames", () => {
    expect(classify("[12:00:00] [Render thread/ERROR]: boom", "stdout")).toBe("error");
    expect(classify("java.lang.NullPointerException: x", "stdout")).toBe("error");
    expect(classify("\tat net.minecraft.Main.main(Main.java:1)", "stdout")).toBe("error");
    expect(classify("some error text", "stderr")).toBe("error");
  });
  it("marks warnings and leaves the rest alone", () => {
    expect(classify("[12:00:00] [main/WARN]: careful", "stdout")).toBe("warn");
    expect(classify("[12:00:00] [main/INFO]: hello", "stdout")).toBe("");
    expect(classify("some error text", "stdout")).toBe("");
  });
});

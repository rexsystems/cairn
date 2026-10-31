// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, waitFor } from "@testing-library/react";
import type { Account } from "../api";
import tauriConf from "../../src-tauri/tauri.conf.json?raw";

const texture = vi.fn<(url: string) => Promise<string>>();
vi.mock("../api", () => ({ api: { texture: (url: string) => texture(url) }, fileSrc: (p?: string | null) => p ?? undefined }));

import { SkinThumb } from "./SkinThumb";
import { CapeThumb } from "./CapeThumb";
import { Avatar } from "./ui";
import { textureSrc } from "../lib/textures";

// Mojang hands out plain http texture links; the CSP only allows https images.
const SKIN = "http://textures.minecraft.net/texture/skin1";
const DATA = "data:image/png;base64,iVBORw0KGgo=";

afterEach(() => {
  cleanup();
  texture.mockReset();
});

describe("skin textures", () => {
  it("are never loaded from Mojang's http host directly", () => {
    const conf = JSON.parse(tauriConf);
    const imgSrc: string = conf.app.security.csp["img-src"];
    expect(imgSrc.split(" ")).not.toContain("http:");
    expect(imgSrc).not.toContain("textures.minecraft.net");
  });

  it("shows the skin face and cape from the backend's data URL", async () => {
    texture.mockImplementation(async () => DATA);
    const { container } = render(
      <>
        <SkinThumb url={SKIN} />
        <CapeThumb url="http://textures.minecraft.net/texture/cape1" />
      </>,
    );
    await waitFor(() => expect(container.querySelectorAll("img")).toHaveLength(2));
    for (const img of container.querySelectorAll("img")) expect(img.getAttribute("src")).toBe(DATA);
    expect(container.innerHTML).not.toContain("http://");
  });

  it("draws the account head from the data URL", async () => {
    texture.mockImplementation(async () => DATA);
    const account: Account = {
      id: "a", uuid: "u", username: "Gogosar", kind: "microsoft", added: 0, active: true, needsSignIn: false,
      skinUrl: "http://textures.minecraft.net/texture/head1",
    };
    const { container } = render(<Avatar account={account} />);
    await waitFor(() => expect((container.querySelector(".face") as HTMLElement).style.backgroundImage).toContain(DATA));
    expect(container.innerHTML).not.toContain("http://textures");
  });

  it("leaves the thumbnail empty instead of a broken image when the download fails", async () => {
    texture.mockRejectedValue("offline");
    const { container } = render(<CapeThumb url="http://textures.minecraft.net/texture/cape2" />);
    await waitFor(() => expect(texture).toHaveBeenCalled());
    await new Promise((r) => setTimeout(r, 0));
    expect(container.querySelector("img")).toBeNull();
  });

  it("downloads each texture once and retries after a failure", async () => {
    texture.mockRejectedValueOnce("offline").mockResolvedValue(DATA);
    const url = "http://textures.minecraft.net/texture/retry";
    await expect(textureSrc(url)).rejects.toBe("offline");
    await expect(Promise.all([textureSrc(url), textureSrc(url)])).resolves.toEqual([DATA, DATA]);
    expect(texture).toHaveBeenCalledTimes(2);
    await expect(textureSrc(DATA)).resolves.toBe(DATA);
    expect(texture).toHaveBeenCalledTimes(2);
  });
});

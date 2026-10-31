import { api, type JoinPlan, type LaunchOpts, type Loader } from "../api";
import { loaderLabel } from "../format";

export type JoinSource = { secret?: string; code?: string };

/** One line telling the player what joining will do before anything is downloaded. */
export function describeJoin(plan: JoinPlan): string {
  const { invite, step } = plan;
  const game = `Minecraft ${invite.mc}${invite.loader === "vanilla" ? "" : `, ${loaderLabel[invite.loader] ?? invite.loader}`}`;
  switch (step.kind) {
    case "launch": return `Starts ${step.instance.name} and joins the server.`;
    case "installPack": return `Installs the same modpack (${game}), then joins.`;
    case "rebuild": return `Creates ${plan.name ?? "the instance"} with the same mods (${game}), then joins.`;
    case "create": return `Creates a ${game} instance, then joins.`;
    case "needCode": return `This server uses mods you don't have (${game}). Ask your friend for the invite code from their Invite friends window and paste it below.`;
  }
}

/** True when the step downloads something, so the player should confirm first. */
export const joinNeedsDownload = (plan: JoinPlan) => plan.step.kind !== "launch" && plan.step.kind !== "needCode";

/** Makes sure the matching instance exists, remembers the session, and starts the game into the server. */
export async function runJoin(
  plan: JoinPlan,
  from: JoinSource,
  play: (id: string, opts?: LaunchOpts) => Promise<void>,
): Promise<string> {
  const { invite, step } = plan;
  let id: string;
  switch (step.kind) {
    case "launch":
      id = step.instance.id;
      break;
    case "installPack": {
      const p = step.pack;
      const inst = p.source === "c" ? await api.curseforgeInstallModpack(p.project, p.version) : await api.installModpack(p.project, p.version);
      id = inst.id;
      break;
    }
    case "rebuild": {
      if (!from.code) throw new Error("The invite code is missing.");
      id = (await api.importShareCode(from.code)).instance.id;
      break;
    }
    case "create": {
      const inst = await api.createInstance({ name: `Minecraft ${invite.mc} server`, mcVersion: invite.mc, loader: invite.loader as Loader, loaderVersion: invite.lv ?? null });
      id = inst.id;
      break;
    }
    case "needCode":
      throw new Error("Paste the invite code from your friend to set up their mods.");
  }
  await api.joinRecord(from, id).catch(() => {});
  await play(id, { server: invite.address });
  return id;
}

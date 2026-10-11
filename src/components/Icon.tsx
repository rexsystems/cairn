import type { ComponentType, SVGProps } from "react";
import { ArrowLeft } from "pixelarticons/react/ArrowLeft";
import { Box } from "pixelarticons/react/Box";
import { Check } from "pixelarticons/react/Check";
import { ChevronDown } from "pixelarticons/react/ChevronDown";
import { Close } from "pixelarticons/react/Close";
import { Coffee } from "pixelarticons/react/Coffee";
import { Compass } from "pixelarticons/react/Compass";
import { Copy } from "pixelarticons/react/Copy";
import { Download } from "pixelarticons/react/Download";
import { ExternalLink } from "pixelarticons/react/ExternalLink";
import { Folder } from "pixelarticons/react/Folder";
import { Globe } from "pixelarticons/react/Globe";
import { Home } from "pixelarticons/react/Home";
import { Image } from "pixelarticons/react/Image";
import { Library } from "pixelarticons/react/Library";
import { MoreHorizontal } from "pixelarticons/react/MoreHorizontal";
import { Pause } from "pixelarticons/react/Pause";
import { Play } from "pixelarticons/react/Play";
import { Plus } from "pixelarticons/react/Plus";
import { Reload } from "pixelarticons/react/Reload";
import { Script } from "pixelarticons/react/Script";
import { Search } from "pixelarticons/react/Search";
import { SettingsCog } from "pixelarticons/react/SettingsCog";
import { SquareAlert } from "pixelarticons/react/SquareAlert";
import { Stop } from "pixelarticons/react/Stop";
import { Trash } from "pixelarticons/react/Trash";
import { Upload } from "pixelarticons/react/Upload";
import { User } from "pixelarticons/react/User";

type SvgIcon = ComponentType<SVGProps<SVGSVGElement>>;

const art = {
  home: Home,
  library: Library,
  discover: Compass,
  user: User,
  settings: SettingsCog,
  pause: Pause,
  play: Play,
  stop: Stop,
  plus: Plus,
  folder: Folder,
  trash: Trash,
  more: MoreHorizontal,
  update: Reload,
  download: Download,
  upload: Upload,
  check: Check,
  close: Close,
  search: Search,
  external: ExternalLink,
  warn: SquareAlert,
  copy: Copy,
  log: Script,
  globe: Globe,
  image: Image,
  cup: Coffee,
  chevron: ChevronDown,
  back: ArrowLeft,
  box: Box,
} satisfies Record<string, SvgIcon>;

export type IconName = keyof typeof art;

export function Icon({ name, size = 18, label }: { name: IconName; size?: number; label?: string }) {
  const Cmp = art[name];
  return (
    <Cmp
      width={size}
      height={size}
      aria-hidden={label ? undefined : true}
      role={label ? "img" : undefined}
      aria-label={label}
      style={{ flex: "none", imageRendering: "pixelated" }}
    />
  );
}

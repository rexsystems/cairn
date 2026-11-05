package app.cairn.client;

import app.cairn.client.CairnClient.Module;

/** Eight-row silhouettes stay crisp at every Minecraft GUI scale. */
final class ModuleIcons {
    static String mask(Module module) {
        switch (module) {
            case FPS: return "7e424a5262427e00";
            case COORDINATES: return "18187e1818244200";
            case PING: return "0000040c2c6c6c00";
            case KEYSTROKES: return "183c180066ff6600";
            case CPS: return "3c5a5a7e42423c00";
            case ARMOR: return "66ff7e3c3c3c3c00";
            case EFFECTS: return "183c183c7e7e3c00";
            case CLOCK: return "3c428a9a82423c00";
            case SPEED: return "4422112244221100";
            case SPRINT: return "0c0c386e18182442";
            case ZOOM: return "38444444380c0603";
            case CROSSHAIR: return "181800c3c3001818";
            case DIRECTION: return "18183c3c7e181800";
            case BIOME: return "18183c3c7e181800";
            case HELD_ITEM: return "03070e1c3870e040";
            case FOOD: return "3c7e7e3c18306000";
            case WORLD_TIME: return "18993c7e7e3c9918";
            case SESSION: return "18183c424a423c00";
            case MEMORY: return "007e5a5a7e242400";
            case SERVER: return "7e427e007e427e00";
            case BLOCK_INDICATOR: return "183c7e5a5a3c1800";
            case REACH: return "000099ff99181800";
            case TPS: return "0024245a5a818100";
            case COMBO: return "03070e5c3870e040";
            case DEATH_INFO: return "3c7edbff7e242400";
            case HORSES: return "183c3e7e7c282800";
            case BOSS_BAR: return "00427eff7e420000";
            case HEARTS: return "0066ffff7e3c1800";
            case ARMOR_POINTS: return "7e7e7e7e3c3c1800";
            case ATTACK_INDICATOR: return "00081c3e7f080800";
            case ELYTRA: return "4281a5a57e3c2400";
            case SHIELD: return "7e424a5a4a241800";
            case TOTEM: return "18183c7e183c2400";
            case LIGHT_LEVEL: return "183c7e7e3c181800";
            case SCOREBOARD: return "7e425a425a427e00";
            case TNT_TIMER: return "18187e5a7e5a7e00";
            case ITEM_INFO: return "1818001818181800";
            case ITEM_DESPAWN: return "7e423c18183c7e00";
            case PACK_DISPLAY: return "183c7e5a5a7e1800";
            case INVENTORY: return "7e5a7e5a7e5a7e00";
            case SYSTEM_RESOURCES: return "247e5a7e5a7e2400";
            case SATURATION: return "3c42427e7e7e3c00";
            case UHC_OVERLAY: return "1866ffff7e3c1800";
            case TEAM_TRACKER: return "2466247e5a5a4200";
            case BLOCK_OVERLAY: return "7e42424242427e00";
            case BRIGHTNESS: return "18993c66663c9918";
            case FOV_CHANGER: return "8142241818244281";
            case HITBOXES: return "7e424a5a52427e00";
            case HIT_INDICATOR: return "8142240000244281";
            case TIME_CHANGER: return "3c428a92425c3e00";
            case WEATHER_CHANGER: return "00386cfe00142800";
            case NICK_HIDER: return "007ec3db7e242400";
            case DROP_PREVENTION: return "3c42427e5a5a7e00";
            case AUTOHIDE_HUD: return "00183c7e3c180000";
            case PERSPECTIVE: return "3c4299a599423c00";
            case SNAPLOOK: return "10207e81817e0000";
            case AUTO_PERSPECTIVE: return "3c4299a599423c18";
            case VIEWMODEL: return "000c1e7e7c381800";
            case PARTICLES: return "1001440010220008";
            case NAMETAGS: return "7e424a425e427e00";
            case RECONNECT: return "3c42800182423c08";
            case TOAST_CONTROL: return "00183c3c7e001800";
            case UI_SCALING: return "e181810000818187";
            case CULL_LOGS: return "7e425a425a663c00";
            case CUSTOM_F3: return "7e42524a52427e00";
            case CHAT_TIMESTAMPS: return "7e425a527e604000";
            case WAYPOINTS: return "3c42665a423c1800";
            case MOB_OVERLAY: return "7e7e5a7e66667e00";
            case LOOT_BEAMS: return "181818183c7e3c18";
            case TITLE_TWEAKER: return "7e42181818181800";
            case FREELOOK: return "3c42819981423c18";
            default: throw new IllegalArgumentException("Missing module icon: " + module);
        }
    }
    static void draw(CairnClient.Canvas canvas, Module module, int x, int y, int size, int color) {
        drawMask(canvas, mask(module), x, y, size, color);
    }
    static void sample(CairnClient.Canvas canvas, String type, int x, int y, int size) {
        String mask = type.equals("helmet") ? "3c7e7e6666420000" : type.equals("chest") ? "66ff7e3c3c3c0000" : type.equals("legs") ? "7e7e666666660000" : type.equals("boots") ? "0066666666ee0000" : type.equals("potion") ? "18183c7e7e3c0000" : "183c7e5a5a3c1800";
        drawMask(canvas, mask, x, y, size, type.equals("potion") ? 0xffdd91e8 : type.equals("block") ? 0xff9fbd70 : 0xff63d9d1);
    }
    private static void drawMask(CairnClient.Canvas canvas, String mask, int x, int y, int size, int color) {
        for (int row = 0; row < 8; row++) {
            int bits = Integer.parseInt(mask.substring(row * 2, row * 2 + 2), 16);
            for (int col = 0; col < 8; col++) if ((bits & (128 >> col)) != 0) {
                int px = x + col * size / 8, py = y + row * size / 8;
                canvas.rect(px, py, (col + 1) * size / 8 - col * size / 8, (row + 1) * size / 8 - row * size / 8, color);
            }
        }
    }
}

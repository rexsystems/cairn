package app.cairn.client.modern;

import app.cairn.client.CairnClient;
import app.cairn.client.CairnClient.Module;
import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.api.ClientModInitializer;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;
import java.util.Locale;

public final class CairnFabric implements ClientModInitializer {
    private static CairnClient client;
    private static boolean menuWasDown, sprintWasDown, sprint;
    private static int ticks;
    private static final java.util.Map<String, Component> TEXT_CACHE = new java.util.LinkedHashMap<String, Component>(128, .75f, true) {
        @Override protected boolean removeEldestEntry(java.util.Map.Entry<String, Component> entry) { return size() > 256; }
    };
    private static Component text(String value) {
        Component component = TEXT_CACHE.get(value);
        if (component == null) { component = Component.literal(value); TEXT_CACHE.put(value, component); }
        return component;
    }
    public static float zoomFactor() {
        return client == null ? 1 : client.zoomFactor(zooming());
    }
    public static boolean hideCrosshair() { return client != null && client.enabled(Module.CROSSHAIR) && client.setting(Module.CROSSHAIR, CairnClient.Setting.CROSS_HIDE_VANILLA) == 1; }
    public void onInitializeClient() {
        if ("false".equals(System.getProperty("cairn.client.enabled"))) return;
        client = new CairnClient(net.fabricmc.loader.api.FabricLoader.getInstance().getGameDir());
    }
    public static boolean zooming() {
        Minecraft mc = Minecraft.getInstance();
        return client != null && client.enabled(Module.ZOOM) && mc.player != null && mc.gui.screen() == null && InputConstants.isKeyDown(InputConstants.KEY_C);
    }
    public static void tick() {
        if (client == null) return;
        Minecraft mc = Minecraft.getInstance();
        boolean menuDown = InputConstants.isKeyDown(InputConstants.KEY_RSHIFT);
        if (menuDown && !menuWasDown && (mc.gui.screen() == null || mc.gui.screen() instanceof net.minecraft.client.gui.screens.TitleScreen)) mc.gui.setScreen(new Menu());
        menuWasDown = menuDown;
        if (mc.player == null) { sprint = false; return; }
        boolean down = mc.options.keySprint.isDown();
        if (client.enabled(Module.SPRINT) && mc.gui.screen() == null) {
            if (down && !sprintWasDown) sprint = !sprint;
            if (client.setting(Module.SPRINT, CairnClient.Setting.SPRINT_AUTO) == 1) sprint = true;
            else if (client.setting(Module.SPRINT, CairnClient.Setting.SPRINT_RESET) == 1 && !mc.options.keyUp.isDown()) sprint = false;
            if (sprint && mc.options.keyUp.isDown() && !mc.player.isShiftKeyDown() && !mc.player.isUsingItem() && !mc.player.horizontalCollision && (mc.player.getFoodData().getFoodLevel() > 6 || mc.player.getAbilities().mayfly)) mc.player.setSprinting(true);
        } else sprint = false;
        sprintWasDown = down;
        client.input(mc.options.keyUp.isDown(), mc.options.keyDown.isDown(), mc.options.keyLeft.isDown(), mc.options.keyRight.isDown(), mc.options.keyJump.isDown(), mc.mouseHandler.isLeftPressed(), mc.mouseHandler.isRightPressed());
        if (ticks++ % 5 != 0) return;
        if (client.enabled(Module.FPS)) client.value(Module.FPS, mc.getFps() + " FPS");
        if (client.enabled(Module.COORDINATES)) client.value(Module.COORDINATES, "XYZ " + (int) Math.floor(mc.player.getX()) + " " + (int) Math.floor(mc.player.getY()) + " " + (int) Math.floor(mc.player.getZ()));
        var info = mc.getConnection() == null ? null : mc.getConnection().getPlayerInfo(mc.player.getUUID());
        if (client.enabled(Module.PING)) client.value(Module.PING, info == null ? "Local world" : info.getLatency() + " ms");
        if (client.enabled(Module.SPEED)) client.value(Module.SPEED, Math.round(Math.hypot(mc.player.getX() - mc.player.xo, mc.player.getZ() - mc.player.zo) * 200) / 10.0 + " blocks/s");
        if (client.enabled(Module.DIRECTION)) client.value(Module.DIRECTION, mc.player.getDirection().toString().toUpperCase(Locale.ROOT));
        if (client.enabled(Module.BIOME)) client.value(Module.BIOME, mc.level.getBiome(mc.player.blockPosition()).unwrapKey().map(key -> key.identifier().getPath().replace('_', ' ')).orElse("Unknown biome"));
        if (client.enabled(Module.FOOD)) client.value(Module.FOOD, "Food " + mc.player.getFoodData().getFoodLevel() + " / Sat " + Math.round(mc.player.getFoodData().getSaturationLevel()));
        if (client.enabled(Module.WORLD_TIME)) { long time = (mc.level.getOverworldClockTime() + 6000) % 24000; client.value(Module.WORLD_TIME, time / 1000 + ":" + (time % 1000 * 60 / 1000 < 10 ? "0" : "") + time % 1000 * 60 / 1000); }
        if (client.enabled(Module.SERVER)) client.value(Module.SERVER, mc.getCurrentServer() == null ? "Singleplayer" : mc.getCurrentServer().ip);
        if (client.enabled(Module.HELD_ITEM)) {
            ItemStack item = mc.player.getMainHandItem();
            client.value(Module.HELD_ITEM, item.isEmpty() ? "Empty hand" : item.getHoverName().getString() + (item.isDamageableItem() ? " " + (item.getMaxDamage() - item.getDamageValue()) : " x" + item.getCount()));
        }
        if (client.enabled(Module.ARMOR)) {
        StringBuilder armor = new StringBuilder("Armor");
        for (EquipmentSlot slot : new EquipmentSlot[] { EquipmentSlot.FEET, EquipmentSlot.LEGS, EquipmentSlot.CHEST, EquipmentSlot.HEAD }) {
            ItemStack item = mc.player.getItemBySlot(slot);
            if (!item.isEmpty()) armor.append(" ").append(item.getMaxDamage() - item.getDamageValue());
        }
        client.value(Module.ARMOR, armor.toString());
        }
        if (client.enabled(Module.EFFECTS)) {
        StringBuilder effects = new StringBuilder();
        for (var effect : mc.player.getActiveEffects()) {
            if (effects.length() > 0) effects.append(" / ");
            effects.append(Component.translatable(effect.getDescriptionId()).getString()).append(" ").append(effect.getDuration() / 20).append("s");
        }
        client.value(Module.EFFECTS, effects.isEmpty() ? "No effects" : effects.toString());
        }
    }
    public static void click(int button) {
        if (client != null && Minecraft.getInstance().gui.screen() == null && (button == 0 || button == 1)) client.click(button == 1);
    }
    private static CairnClient.Canvas canvas(GuiGraphicsExtractor graphics) {
        return new CairnClient.Canvas() {
            public void rect(int x, int y, int width, int height, int color) { graphics.fill(x, y, x + width, y + height, color); }
            public void text(String value, int x, int y, int color) { graphics.text(Minecraft.getInstance().font, CairnFabric.text(value), x, y, color, false); }
            public void textScaled(String value, int x, int y, int color, float scale, boolean shadow) {
                graphics.pose().pushMatrix(); graphics.pose().translate(x, y).scale(scale, scale);
                graphics.text(Minecraft.getInstance().font, CairnFabric.text(value), 0, 0, color, shadow);
                graphics.pose().popMatrix();
            }
            public int textWidth(String value) { return Minecraft.getInstance().font.width(CairnFabric.text(value)); }
        };
    }
    public static void hud(GuiGraphicsExtractor graphics) {
        Minecraft mc = Minecraft.getInstance();
        if (client != null && mc.player != null && !mc.gui.hud.isHidden() && !(mc.gui.screen() instanceof Menu)) client.renderHud(canvas(graphics), graphics.guiWidth(), graphics.guiHeight());
    }
    private static final class Menu extends Screen {
        Menu() { super(Component.literal("Cairn Client")); }
        @Override public void extractRenderState(GuiGraphicsExtractor graphics, int x, int y, float delta) { client.renderMenu(canvas(graphics), width, height, x, y); }
        @Override public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) { if (event.button() == 0) { client.mouseDown(width, height, (int) event.x(), (int) event.y()); return true; } return false; }
        @Override public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) { if (event.button() == 0) { client.drag(width, height, (int) event.x(), (int) event.y()); return true; } return false; }
        @Override public boolean mouseReleased(MouseButtonEvent event) { client.mouseUp(); return true; }
        @Override public boolean mouseScrolled(double x, double y, double horizontal, double vertical) { client.wheel(vertical < 0 ? 1 : -1); return true; }
        @Override public boolean keyPressed(KeyEvent event) {
            int key = event.key();
            if (key == InputConstants.KEY_ESCAPE || key == InputConstants.KEY_RSHIFT) { if (!client.escape()) onClose(); return true; }
            if (client.typing()) {
                if (key == InputConstants.KEY_BACKSPACE) client.backspace();
                else if (key == InputConstants.KEY_RETURN) client.key(0, true);
                return true;
            }
            if (key == InputConstants.KEY_TAB) { client.tab(); return true; }
            if (client.editing()) {
                if (key == InputConstants.KEY_G) client.snap();
                else if (key == InputConstants.KEY_H) client.toggleEditorTools();
                else if (key == InputConstants.KEY_RETURN) client.editorSettings();
                else if (key == InputConstants.KEY_EQUALS || key == InputConstants.KEY_ADD) client.editorKey(0, 0, 1);
                else if (key == InputConstants.KEY_MINUS) client.editorKey(0, 0, -1);
                else if (key == InputConstants.KEY_RIGHT) client.editorKey(1, 0, 0);
                else if (key == InputConstants.KEY_LEFT) client.editorKey(-1, 0, 0);
                else if (key == InputConstants.KEY_UP) client.editorKey(0, -1, 0);
                else if (key == InputConstants.KEY_DOWN) client.editorKey(0, 1, 0);
                return true;
            }
            if (key == InputConstants.KEY_F && event.hasControlDown()) { client.focusSearch(); return true; }
            if (key == InputConstants.KEY_RETURN || key == InputConstants.KEY_SPACE) { client.key(0, true); return true; }
            if (key == InputConstants.KEY_E) { client.enterEditor(); return true; }
            if (key == InputConstants.KEY_RIGHT || key == InputConstants.KEY_DOWN) { client.key(1, false); return true; }
            if (key == InputConstants.KEY_LEFT || key == InputConstants.KEY_UP) { client.key(-1, false); return true; }
            return super.keyPressed(event);
        }
        @Override public boolean charTyped(net.minecraft.client.input.CharacterEvent event) {
            if (client.typing() && event.codepoint() <= Character.MAX_VALUE) { client.type((char) event.codepoint()); return true; }
            return false;
        }
        @Override public void onClose() { minecraft.gui.setScreen(null); }
        @Override public void removed() { client.leaveEditor(); }
        @Override public boolean isPauseScreen() { return false; }
    }
}

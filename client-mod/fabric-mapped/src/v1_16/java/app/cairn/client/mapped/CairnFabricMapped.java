package app.cairn.client.mapped;

import app.cairn.client.CairnClient;
import app.cairn.client.CairnClient.Module;
import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.api.ClientModInitializer;
import org.lwjgl.glfw.GLFW;
import net.minecraft.client.Minecraft;
import com.mojang.blaze3d.vertex.PoseStack;
import net.minecraft.client.gui.GuiComponent;
import net.minecraft.client.resources.language.I18n;
import net.minecraft.core.Registry;
import net.minecraft.network.chat.TextComponent;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;
import java.util.Locale;

public final class CairnFabricMapped implements ClientModInitializer {
    private static CairnClient client;
    private static boolean sprintWasDown, sprint;
    private static int ticks;
    private static final java.util.Map<String, Component> TEXT_CACHE = new java.util.LinkedHashMap<String, Component>(128, .75f, true) {
        @Override protected boolean removeEldestEntry(java.util.Map.Entry<String, Component> entry) { return size() > 256; }
    };
    private static Component text(String value) {
        Component component = TEXT_CACHE.get(value);
        if (component == null) { component = new TextComponent(value); TEXT_CACHE.put(value, component); }
        return component;
    }
    public void onInitializeClient() {
        if ("false".equals(System.getProperty("cairn.client.enabled"))) return;
        client = new CairnClient(net.fabricmc.loader.api.FabricLoader.getInstance().getGameDir());
        client.manageKeys(GLFW.GLFW_KEY_RIGHT_SHIFT, "Right Shift", GLFW.GLFW_KEY_C, "C");
    }
    private static boolean key(int code) { return InputConstants.isKeyDown(Minecraft.getInstance().getWindow().getWindow(), code); }
    private static boolean zooming() {
        Minecraft mc = Minecraft.getInstance();
        return client != null && mc.player != null && mc.screen == null && key(client.keyCode(CairnClient.Bind.ZOOM));
    }
    public static double zoomFactor() { return client == null ? 1 : client.zoomFactor(zooming()); }
    /** Key events arrive between ticks, so a quick tap of the menu key is never missed. */
    public static boolean keyPress(int key, int action) {
        Minecraft mc = Minecraft.getInstance();
        if (client == null || action != 1 || !client.menuKey(key) || !(mc.screen == null || mc.screen instanceof net.minecraft.client.gui.screens.TitleScreen)) return false;
        mc.setScreen(new Menu());
        return true;
    }
    public static boolean scroll(double amount) { return client != null && client.zoomScroll(zooming(), amount); }
    public static boolean hideCrosshair() { return client != null && client.enabled(Module.CROSSHAIR) && client.setting(Module.CROSSHAIR, CairnClient.Setting.CROSS_HIDE_VANILLA) == 1; }
    private static void input() {
        Minecraft mc = Minecraft.getInstance();
        client.input(mc.options.keyUp.isDown(), mc.options.keyDown.isDown(), mc.options.keyLeft.isDown(), mc.options.keyRight.isDown(), mc.options.keyJump.isDown(), mc.mouseHandler.isLeftPressed(), mc.mouseHandler.isRightPressed());
    }
    public static void tick() {
        if (client == null) return;
        Minecraft mc = Minecraft.getInstance();
        mc.options.smoothCamera = client.smoothCamera(zooming(), mc.options.smoothCamera);
        if (mc.player == null) { sprint = false; return; }
        boolean down = mc.options.keySprint.isDown();
        if (client.enabled(Module.SPRINT) && mc.screen == null) {
            if (down && !sprintWasDown) sprint = !sprint;
            if (client.setting(Module.SPRINT, CairnClient.Setting.SPRINT_AUTO) == 1) sprint = true;
            else if (client.setting(Module.SPRINT, CairnClient.Setting.SPRINT_RESET) == 1 && !mc.options.keyUp.isDown()) sprint = false;
            if (sprint && mc.options.keyUp.isDown() && !mc.player.isShiftKeyDown() && !mc.player.isUsingItem() && !mc.player.horizontalCollision && (mc.player.getFoodData().getFoodLevel() > 6 || mc.player.abilities.mayfly)) mc.player.setSprinting(true);
        } else sprint = false;
        sprintWasDown = down;
        input();
        if (ticks++ % 5 != 0) return;
        // 1.16 keeps the frame counter private; its debug string starts with the same number.
        if (client.enabled(Module.FPS)) client.value(Module.FPS, mc.fpsString.split(" ", 2)[0] + " FPS");
        if (client.enabled(Module.COORDINATES)) client.value(Module.COORDINATES, "XYZ " + (int) Math.floor(mc.player.getX()) + " " + (int) Math.floor(mc.player.getY()) + " " + (int) Math.floor(mc.player.getZ()));
        net.minecraft.client.multiplayer.PlayerInfo info = mc.getConnection() == null ? null : mc.getConnection().getPlayerInfo(mc.player.getUUID());
        if (client.enabled(Module.PING)) client.value(Module.PING, info == null ? "Local world" : info.getLatency() + " ms");
        if (client.enabled(Module.SPEED)) client.value(Module.SPEED, Math.round(Math.hypot(mc.player.getX() - mc.player.xo, mc.player.getZ() - mc.player.zo) * 200) / 10.0 + " blocks/s");
        if (client.enabled(Module.DIRECTION)) client.value(Module.DIRECTION, mc.player.getDirection().toString().toUpperCase(Locale.ROOT));
        if (client.enabled(Module.BIOME)) client.value(Module.BIOME, biome(mc));
        if (client.enabled(Module.FOOD)) client.value(Module.FOOD, "Food " + mc.player.getFoodData().getFoodLevel() + " / Sat " + Math.round(mc.player.getFoodData().getSaturationLevel()));
        if (client.enabled(Module.WORLD_TIME)) { long time = (mc.level.getDayTime() + 6000) % 24000; client.value(Module.WORLD_TIME, time / 1000 + ":" + (time % 1000 * 60 / 1000 < 10 ? "0" : "") + time % 1000 * 60 / 1000); }
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
            for (net.minecraft.world.effect.MobEffectInstance effect : mc.player.getActiveEffects()) {
                if (effects.length() > 0) effects.append(" / ");
                effects.append(I18n.get(effect.getDescriptionId())).append(" ").append(effect.getDuration() / 20).append("s");
            }
            client.value(Module.EFFECTS, effects.length() == 0 ? "No effects" : effects.toString());
        }
    }
    private static String biome(Minecraft mc) {
        ResourceLocation key = mc.level.registryAccess().registryOrThrow(Registry.BIOME_REGISTRY).getKey(mc.level.getBiome(mc.player.blockPosition()));
        return key == null ? "Unknown biome" : key.getPath().replace('_', ' ');
    }
    public static void click(int button) {
        if (client != null && Minecraft.getInstance().screen == null && (button == 0 || button == 1)) client.click(button == 1);
    }
    private static CairnClient.Canvas canvas(PoseStack pose) {
        return new CairnClient.Canvas() {
            public void rect(int x, int y, int width, int height, int color) { GuiComponent.fill(pose, x, y, x + width, y + height, color); }
            public void text(String value, int x, int y, int color) { Minecraft.getInstance().font.draw(pose, CairnFabricMapped.text(value), x, y, color); }
            public void textScaled(String value, int x, int y, int color, float scale, boolean shadow) {
                pose.pushPose(); pose.translate(x, y, 0); pose.scale(scale, scale, 1);
                if (shadow) Minecraft.getInstance().font.drawShadow(pose, CairnFabricMapped.text(value), 0, 0, color);
                else Minecraft.getInstance().font.draw(pose, CairnFabricMapped.text(value), 0, 0, color);
                pose.popPose();
            }
            public int textWidth(String value) { return Minecraft.getInstance().font.width(CairnFabricMapped.text(value)); }
        };
    }
    public static void hud(PoseStack graphics) {
        Minecraft mc = Minecraft.getInstance();
        if (client != null && mc.player != null && !mc.options.hideGui && !(mc.screen instanceof Menu)) {
            if (client.enabled(Module.KEYSTROKES)) input();
            client.renderHud(canvas(graphics), mc.getWindow().getGuiScaledWidth(), mc.getWindow().getGuiScaledHeight());
        }
    }
    private static final class Menu extends Screen {
        Menu() { super(new TextComponent("Cairn Client")); }
        @Override public void render(PoseStack graphics, int x, int y, float delta) { client.renderMenu(canvas(graphics), width, height, x, y); }
        @Override public boolean mouseClicked(double x, double y, int button) { if (button == 0) { client.mouseDown(width, height, (int) x, (int) y); return true; } return false; }
        @Override public boolean mouseDragged(double x, double y, int button, double dx, double dy) { if (button == 0) { client.drag(width, height, (int) x, (int) y); return true; } return false; }
        @Override public boolean mouseReleased(double x, double y, int button) { client.mouseUp(); return true; }
        @Override public boolean mouseScrolled(double x, double y, double amount) { client.wheel(amount < 0 ? 1 : -1); return true; }
        @Override public boolean keyPressed(int key, int scanCode, int modifiers) {
            if (client.binding()) { client.bind(key == GLFW.GLFW_KEY_ESCAPE ? -1 : key, InputConstants.Type.KEYSYM.getOrCreate(key).getDisplayName().getString()); return true; }
            if (key == GLFW.GLFW_KEY_ESCAPE || key == client.keyCode(CairnClient.Bind.MENU)) { if (!client.escape()) onClose(); return true; }
            if (client.typing()) {
                if (key == GLFW.GLFW_KEY_BACKSPACE) client.backspace(); else if (key == GLFW.GLFW_KEY_ENTER) client.key(0, true);
                return true;
            }
            if (key == GLFW.GLFW_KEY_TAB) { client.tab(); return true; }
            if (client.editing()) {
                if (key == GLFW.GLFW_KEY_G) client.snap();
                else if (key == GLFW.GLFW_KEY_H) client.toggleEditorTools();
                else if (key == GLFW.GLFW_KEY_ENTER) client.editorSettings();
                else if (key == GLFW.GLFW_KEY_EQUAL || key == GLFW.GLFW_KEY_KP_ADD) client.editorKey(0, 0, 1);
                else if (key == GLFW.GLFW_KEY_MINUS) client.editorKey(0, 0, -1);
                else if (key == GLFW.GLFW_KEY_RIGHT) client.editorKey(1, 0, 0);
                else if (key == GLFW.GLFW_KEY_LEFT) client.editorKey(-1, 0, 0);
                else if (key == GLFW.GLFW_KEY_UP) client.editorKey(0, -1, 0);
                else if (key == GLFW.GLFW_KEY_DOWN) client.editorKey(0, 1, 0);
                return true;
            }
            if (key == GLFW.GLFW_KEY_F && hasControlDown()) { client.focusSearch(); return true; }
            if (key == GLFW.GLFW_KEY_ENTER || key == GLFW.GLFW_KEY_SPACE) { client.key(0, true); return true; }
            if (key == GLFW.GLFW_KEY_E) { client.enterEditor(); return true; }
            if (key == GLFW.GLFW_KEY_RIGHT || key == GLFW.GLFW_KEY_DOWN) { client.key(1, false); return true; }
            if (key == GLFW.GLFW_KEY_LEFT || key == GLFW.GLFW_KEY_UP) { client.key(-1, false); return true; }
            return super.keyPressed(key, scanCode, modifiers);
        }
        @Override public boolean charTyped(char character, int modifiers) { if (client.typing()) { client.type(character); return true; } return false; }
        @Override public void onClose() { minecraft.setScreen(null); }
        @Override public void removed() { client.leaveEditor(); }
        @Override public boolean isPauseScreen() { return false; }
    }
}

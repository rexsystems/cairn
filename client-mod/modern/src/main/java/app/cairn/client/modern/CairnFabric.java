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
import app.cairn.client.Cosmetics;
import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.vertex.PoseStack;
import net.minecraft.client.player.AbstractClientPlayer;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.state.AvatarRenderState;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.core.ClientAsset;
import net.minecraft.resources.Identifier;
import net.minecraft.util.LightCoordsUtil;
import net.minecraft.world.entity.Avatar;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.PlayerSkin;
import net.minecraft.world.phys.Vec3;

public final class CairnFabric implements ClientModInitializer {
    private static CairnClient client;
    private static boolean sprintWasDown, sprint;
    private static int ticks;
    private static final java.util.Map<String, Component> TEXT_CACHE = new java.util.LinkedHashMap<String, Component>(128, .75f, true) {
        @Override protected boolean removeEldestEntry(java.util.Map.Entry<String, Component> entry) { return size() > 256; }
    };
    private static Component text(String value) {
        Component component = TEXT_CACHE.get(value);
        if (component == null) { component = Component.literal(value); TEXT_CACHE.put(value, component); }
        return component;
    }
    private static final java.util.Map<String, Identifier> TEXTURES = new java.util.HashMap<String, Identifier>();
    private static final java.util.Map<Identifier, ClientAsset.ResourceTexture> CAPE_ASSETS = new java.util.HashMap<Identifier, ClientAsset.ResourceTexture>();
    /** Uploads cosmetics PNGs lazily on the render thread; a broken image is remembered so it isn't decoded every frame. */
    private static Identifier texture(String key) {
        if ("preview/world".equals(key)) return Identifier.withDefaultNamespace("textures/gui/title/background/panorama_0.png");
        if (key != null && key.startsWith("effect/")) { String[] parts = key.split("/", 3); return Identifier.fromNamespaceAndPath(parts[1], "textures/mob_effect/" + parts[2] + ".png"); }
        if (client == null || key == null) return null;
        if (TEXTURES.containsKey(key)) return TEXTURES.get(key);
        byte[] png = client.cosmetics.texture(key);
        if (png == null) return null;
        Identifier id = Identifier.fromNamespaceAndPath("cairn_client", "cosmetics/" + key);
        try {
            NativeImage image = NativeImage.read(png);
            Minecraft.getInstance().getTextureManager().register(id, new DynamicTexture(id::toString, image));
        } catch (java.io.IOException | RuntimeException e) {
            System.err.println("[Cairn Client] Could not upload " + key + ": " + e);
            id = null;
        }
        TEXTURES.put(key, id);
        return id;
    }
    private static Cosmetics.Look look(Entity entity) { return client == null || entity == null ? null : client.cosmetics.look(entity.getUUID().toString()); }
    public static void dress(Avatar entity, AvatarRenderState state) {
        if (!(entity instanceof AbstractClientPlayer)) return;
        Cosmetics.Look look = look(entity);
        if (look == null) return;
        Identifier cape = texture(client.cosmetics.capeTexture(look));
        if (cape != null && state.skin != null) {
            PlayerSkin skin = state.skin;
            state.skin = new PlayerSkin(skin.body(), CAPE_ASSETS.computeIfAbsent(cape, id -> new ClientAsset.ResourceTexture(id, id)), skin.elytra(), skin.model(), skin.secure());
        }
    }
    /** Mirrors vanilla's name tag transform so the logo badge sits just left of the name. */
    public static void badge(AvatarRenderState state, PoseStack pose, SubmitNodeCollector collector, CameraRenderState camera) {
        Minecraft mc = Minecraft.getInstance();
        if (client == null || state.nameTag == null || state.nameTagAttachment == null || mc.level == null) return;
        Identifier icon = texture(client.cosmetics.badgeTexture(look(mc.level.getEntity(state.id))));
        if (icon == null) return;
        float x0 = -mc.font.width(state.nameTag) / 2f - 11, x1 = x0 + 9, y0 = (state.showExtraEars ? -10 : 0) - .5f, y1 = y0 + 9;
        int light = LightCoordsUtil.lightCoordsWithEmission(state.lightCoords, 2);
        Vec3 at = state.nameTagAttachment;
        pose.pushPose();
        if (state.scoreText != null) pose.translate(0, 9 * 1.15f * .025f, 0);
        pose.translate(at.x, at.y + .5, at.z);
        pose.rotate(camera.orientation);
        pose.scale(.025f, -.025f, .025f);
        collector.submitCustomGeometry(pose, RenderTypes.text(icon), (matrix, buffer) -> {
            buffer.addVertex(matrix, x0, y0, 0).setColor(-1).setUv(0, 0).setLight(light);
            buffer.addVertex(matrix, x0, y1, 0).setColor(-1).setUv(0, 1).setLight(light);
            buffer.addVertex(matrix, x1, y1, 0).setColor(-1).setUv(1, 1).setLight(light);
            buffer.addVertex(matrix, x1, y0, 0).setColor(-1).setUv(1, 0).setLight(light);
        });
        pose.popPose();
    }
    public static float zoomFactor() {
        return client == null ? 1 : client.zoomFactor(zooming());
    }
    public static boolean hideCrosshair() { return client != null && client.enabled(Module.CROSSHAIR) && client.setting(Module.CROSSHAIR, CairnClient.Setting.CROSS_HIDE_VANILLA) == 1; }
    public void onInitializeClient() {
        if ("false".equals(System.getProperty("cairn.client.enabled"))) return;
        client = new CairnClient(net.fabricmc.loader.api.FabricLoader.getInstance().getGameDir());
        client.manageKeys(InputConstants.KEY_RSHIFT, "Right Shift", InputConstants.KEY_C, "C");
    }
    /** Key events arrive between ticks, so a quick tap of the menu key is never missed. */
    public static boolean keyPress(int key, int action) {
        Minecraft mc = Minecraft.getInstance();
        Screen screen = mc.gui.screen();
        if (client == null || action != 1 || !client.menuKey(key) || !(screen == null || screen instanceof net.minecraft.client.gui.screens.TitleScreen)) return false;
        mc.gui.setScreen(new Menu());
        return true;
    }
    public static boolean scroll(double amount) { return client != null && client.zoomScroll(zooming(), amount); }
    public static boolean zooming() {
        Minecraft mc = Minecraft.getInstance();
        return client != null && client.enabled(Module.ZOOM) && mc.player != null && mc.gui.screen() == null && InputConstants.isKeyDown(client.keyCode(CairnClient.Bind.ZOOM));
    }
    private static final String GAME_VERSION = net.fabricmc.loader.api.FabricLoader.getInstance().getModContainer("minecraft").map(c -> c.getMetadata().getVersion().getFriendlyString()).orElse("unknown");
    private static final java.util.function.Consumer<String> JOIN = address -> {
        Minecraft mc = Minecraft.getInstance();
        if (mc.level != null) return;
        net.minecraft.client.gui.screens.ConnectScreen.startConnecting(new net.minecraft.client.gui.screens.TitleScreen(), mc,
            net.minecraft.client.multiplayer.resolver.ServerAddress.parseString(address), new net.minecraft.client.multiplayer.ServerData(address, address, net.minecraft.client.multiplayer.ServerData.Type.OTHER), false, null);
    };
    public static void tick() {
        if (client == null) return;
        Minecraft mc = Minecraft.getInstance();
        if (mc.gui.screen() != null && mc.gui.screen().getClass() == net.minecraft.client.gui.screens.TitleScreen.class) mc.gui.setScreen(new Title());
        mc.options.smoothCamera = client.smoothCamera(zooming(), mc.options.smoothCamera);
        client.cosmetics.account(mc.getUser().getName(), mc.getUser().getProfileId().toString(), mc.getUser().getAccessToken());
        if (mc.level != null && ticks % 20 == 0) for (AbstractClientPlayer player : mc.level.players()) look(player);
        client.cosmetics.tick();
        client.friends.activity(GAME_VERSION, mc.level != null, mc.level == null || mc.getCurrentServer() == null ? null : mc.getCurrentServer().ip);
        // Joining from inside a world would need a disconnect first, so Join is offered from the title screen only.
        client.friends.joinHandler(mc.level == null ? JOIN : null);
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
        java.util.List<CairnClient.HudRow> rows = new java.util.ArrayList<>();
        StringBuilder armor = new StringBuilder("Armor");
        for (EquipmentSlot slot : new EquipmentSlot[] { EquipmentSlot.FEET, EquipmentSlot.LEGS, EquipmentSlot.CHEST, EquipmentSlot.HEAD }) {
            ItemStack item = mc.player.getItemBySlot(slot);
            if (!item.isEmpty()) { armor.append(" ").append(item.getMaxDamage() - item.getDamageValue()); rows.add(new CairnClient.HudRow(item.copy(), String.valueOf(item.getMaxDamage() - item.getDamageValue()))); }
        }
        client.value(Module.ARMOR, armor.toString());
        client.rows(Module.ARMOR, rows);
        }
        if (client.enabled(Module.EFFECTS)) {
        java.util.List<CairnClient.HudRow> rows = new java.util.ArrayList<>();
        StringBuilder effects = new StringBuilder();
        for (var effect : mc.player.getActiveEffects()) {
            if (effects.length() > 0) effects.append(" / ");
            effects.append(Component.translatable(effect.getDescriptionId()).getString()).append(" ").append(effect.getDuration() / 20).append("s");
            rows.add(new CairnClient.HudRow(CairnClient.effectIcon(effect.getDescriptionId()), Component.translatable(effect.getDescriptionId()).getString() + " " + (effect.getAmplifier() + 1) + " / " + effect.getDuration() / 20 + "s"));
        }
        client.value(Module.EFFECTS, effects.isEmpty() ? "No effects" : effects.toString());
        client.rows(Module.EFFECTS, rows);
        }
    }
    public static void click(int button) {
        if (client != null && Minecraft.getInstance().gui.screen() == null) client.mouseClick(button, InputConstants.MOUSE_BUTTON_LEFT, InputConstants.MOUSE_BUTTON_RIGHT);
    }
    private static CairnClient.Canvas canvas(GuiGraphicsExtractor graphics) {
        return new CairnClient.Canvas() {
            public void rect(int x, int y, int width, int height, int color) { graphics.fill(x, y, x + width, y + height, color); }
            public void text(String value, int x, int y, int color) { graphics.text(Minecraft.getInstance().font, CairnFabric.text(value), x, y, color, false); }
            public void textScaled(String value, int x, int y, int color, float scale, boolean shadow) {
                graphics.nextStratum();
                graphics.pose().pushMatrix(); graphics.pose().translate(x, y).scale(scale, scale);
                graphics.text(Minecraft.getInstance().font, CairnFabric.text(value), 0, 0, color, shadow);
                graphics.pose().popMatrix();
                graphics.nextStratum();
            }
            public int textWidth(String value) { return Minecraft.getInstance().font.width(CairnFabric.text(value)); }
            public void icon(Object icon, int x, int y, int size) {
                if (!(icon instanceof ItemStack item)) { CairnClient.Canvas.super.icon(icon, x, y, size); return; }
                graphics.nextStratum(); graphics.pose().pushMatrix(); graphics.pose().translate(x, y).scale(size / 16f, size / 16f);
                graphics.item(item, 0, 0); graphics.pose().popMatrix(); graphics.nextStratum();
            }
            public void image(String key, int x, int y, int width, int height, float u0, float v0, float u1, float v1) {
                Identifier id = texture(key);
                if (id != null) graphics.blit(id, x, y, x + width, y + height, u0, u1, v0, v1);
            }
        };
    }
    public static void hud(GuiGraphicsExtractor graphics) {
        Minecraft mc = Minecraft.getInstance();
        if (client != null && mc.player != null && !mc.gui.hud.isHidden() && !(mc.gui.screen() instanceof Menu)) client.renderHud(canvas(graphics), graphics.guiWidth(), graphics.guiHeight());
    }
    private static final class Title extends net.minecraft.client.gui.screens.TitleScreen {
        private app.cairn.client.CairnTitle view;
        @Override protected void init() {
            super.init();
            view = new app.cairn.client.CairnTitle();
            nativeAction("menu.singleplayer", "Singleplayer"); nativeAction("menu.multiplayer", "Multiplayer");
            view.add("Cairn Settings", () -> { minecraft.gui.setScreen(new Menu()); client.openModules(); }, true);
            view.add("Cosmetics", () -> { minecraft.gui.setScreen(new Menu()); client.openCosmetics(); }, true);
            view.add("Friends", () -> { minecraft.gui.setScreen(new Menu()); client.openFriends(); }, true);
            nativeAction("menu.options", "Minecraft settings"); nativeAction("menu.quit", "Quit game");
            client.cosmetics.badgeTexture("classic");
        }
        private void nativeAction(String key, String label) {
            for (var child : children()) if (child instanceof net.minecraft.client.gui.components.Button button && button.getMessage().getString().equals(Component.translatable(key).getString()))
                view.add(label, () -> button.onPress(new KeyEvent(InputConstants.KEY_RETURN, 0, 0)), button.active);
        }
        @Override public void extractRenderState(GuiGraphicsExtractor graphics, int x, int y, float delta) {
            extractPanorama(graphics, delta); view.render(canvas(graphics), width, height, x, y);
        }
        @Override public boolean mouseClicked(MouseButtonEvent event, boolean twice) { if (event.button() == InputConstants.MOUSE_BUTTON_LEFT) view.click((int) event.x(), (int) event.y()); return true; }
        @Override public boolean keyPressed(KeyEvent event) {
            if (client.menuKey(event.key())) { minecraft.gui.setScreen(new Menu()); return true; }
            if (event.key() == InputConstants.KEY_TAB || event.key() == InputConstants.KEY_DOWN || event.key() == InputConstants.KEY_UP) { view.key(false, event.key() == InputConstants.KEY_UP || event.hasShiftDown()); return true; }
            if (event.key() == InputConstants.KEY_RETURN || event.key() == InputConstants.KEY_SPACE) { view.key(true, false); return true; }
            return false;
        }
    }
    private static final class Menu extends Screen {
        Menu() { super(Component.literal("Cairn Client")); client.enterEditor(); }
        @Override public void tick() { if (client.consumeMenuClose()) onClose(); }
        @Override public void extractRenderState(GuiGraphicsExtractor graphics, int x, int y, float delta) {
            client.renderMenu(canvas(graphics), width, height, x, y);
            minecraft.textInputManager().onTextInputFocusChange(this, client.typing());
        }
        @Override public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) { if (event.button() == InputConstants.MOUSE_BUTTON_LEFT) { client.mouseDown(width, height, (int) event.x(), (int) event.y()); return true; } return false; }
        @Override public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) { if (event.button() == InputConstants.MOUSE_BUTTON_LEFT) { client.drag(width, height, (int) event.x(), (int) event.y()); return true; } return false; }
        @Override public boolean mouseReleased(MouseButtonEvent event) { client.mouseUp(); return true; }
        @Override public boolean mouseScrolled(double x, double y, double horizontal, double vertical) { client.wheel(vertical < 0 ? 1 : -1); return true; }
        @Override public boolean keyPressed(KeyEvent event) {
            int key = event.key();
            if (client.binding()) { client.bind(key == InputConstants.KEY_ESCAPE ? -1 : key, InputConstants.getKey(event).getDisplayName().getString()); return true; }
            if (key == InputConstants.KEY_ESCAPE || client.menuCloseKey(key, InputConstants.KEY_TAB)) { if (!client.escape()) onClose(); return true; }
            if (key == InputConstants.KEY_TAB) { client.tab(); return true; }
            if (client.typing()) {
                if (key == InputConstants.KEY_BACKSPACE) client.backspace();
                else if (key == InputConstants.KEY_RETURN) client.key(0, true);
                return true;
            }
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
        @Override public void removed() { minecraft.textInputManager().stopTextInput(this); client.leaveEditor(); }
        @Override public boolean isPauseScreen() { return false; }
    }
}

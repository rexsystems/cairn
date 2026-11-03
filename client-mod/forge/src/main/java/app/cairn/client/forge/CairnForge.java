package app.cairn.client.forge;

import app.cairn.client.CairnClient;
import app.cairn.client.CairnClient.Module;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Gui;
import net.minecraft.client.gui.GuiScreen;
import net.minecraft.client.settings.KeyBinding;
import net.minecraft.item.ItemStack;
import net.minecraft.potion.PotionEffect;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.client.event.FOVUpdateEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.client.registry.ClientRegistry;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.event.FMLInitializationEvent;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.InputEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.input.Keyboard;
import org.lwjgl.input.Mouse;
import java.io.IOException;
import java.util.Locale;

@Mod(modid = "cairn_client", name = "Cairn Client", version = "0.1.0", clientSideOnly = true, acceptedMinecraftVersions = "[1.8,1.8.9]")
public final class CairnForge {
    private final Minecraft mc = Minecraft.getMinecraft();
    private final KeyBinding menu = new KeyBinding("Cairn menu", Keyboard.KEY_RSHIFT, "Cairn Client");
    private final KeyBinding zoom = new KeyBinding("Cairn zoom", Keyboard.KEY_C, "Cairn Client");
    private CairnClient client;
    private boolean sprint;
    private boolean sprintWasDown;
    private int ticks;

    @Mod.EventHandler public void init(FMLInitializationEvent event) {
        if ("false".equals(System.getProperty("cairn.client.enabled"))) return;
        client = new CairnClient(mc.mcDataDir.toPath());
        ClientRegistry.registerKeyBinding(menu);
        ClientRegistry.registerKeyBinding(zoom);
        MinecraftForge.EVENT_BUS.register(this);
        net.minecraftforge.fml.common.FMLCommonHandler.instance().bus().register(this);
    }
    @SubscribeEvent public void key(InputEvent.KeyInputEvent event) {
        if (menu.isPressed() && mc.currentScreen == null) mc.displayGuiScreen(new Menu());
    }
    @SubscribeEvent public void menuKey(net.minecraftforge.client.event.GuiScreenEvent.KeyboardInputEvent.Post event) {
        if (mc.currentScreen instanceof net.minecraft.client.gui.GuiMainMenu && Keyboard.getEventKeyState() && Keyboard.getEventKey() == menu.getKeyCode()) mc.displayGuiScreen(new Menu());
    }
    @SubscribeEvent public void tick(TickEvent.ClientTickEvent event) {
        if (event.phase != TickEvent.Phase.END || mc.thePlayer == null) return;
        boolean left = Mouse.isButtonDown(0), right = Mouse.isButtonDown(1);
        boolean down = mc.gameSettings.keyBindSprint.isKeyDown();
        if (client.enabled(Module.SPRINT) && mc.currentScreen == null) {
            if (down && !sprintWasDown) sprint = !sprint;
            if (client.setting(Module.SPRINT, CairnClient.Setting.SPRINT_AUTO) == 1) sprint = true;
            else if (client.setting(Module.SPRINT, CairnClient.Setting.SPRINT_RESET) == 1 && mc.thePlayer.movementInput.moveForward <= 0) sprint = false;
            if (sprint && mc.thePlayer.movementInput.moveForward > 0 && !mc.thePlayer.isSneaking() && !mc.thePlayer.isUsingItem() && !mc.thePlayer.isCollidedHorizontally && (mc.thePlayer.getFoodStats().getFoodLevel() > 6 || mc.thePlayer.capabilities.allowFlying)) mc.thePlayer.setSprinting(true);
        } else sprint = false;
        sprintWasDown = down;
        client.input(mc.gameSettings.keyBindForward.isKeyDown(), mc.gameSettings.keyBindBack.isKeyDown(), mc.gameSettings.keyBindLeft.isKeyDown(), mc.gameSettings.keyBindRight.isKeyDown(), mc.gameSettings.keyBindJump.isKeyDown(), left, right);
        if (ticks++ % 5 != 0) return;
        if (client.enabled(Module.FPS)) client.value(Module.FPS, Minecraft.getDebugFPS() + " FPS");
        if (client.enabled(Module.COORDINATES)) client.value(Module.COORDINATES, "XYZ " + (int) Math.floor(mc.thePlayer.posX) + " " + (int) Math.floor(mc.thePlayer.posY) + " " + (int) Math.floor(mc.thePlayer.posZ));
        if (client.enabled(Module.PING)) client.value(Module.PING, mc.getNetHandler() != null && mc.getNetHandler().getPlayerInfo(mc.thePlayer.getUniqueID()) != null ? mc.getNetHandler().getPlayerInfo(mc.thePlayer.getUniqueID()).getResponseTime() + " ms" : "Local world");
        if (client.enabled(Module.SPEED)) client.value(Module.SPEED, Math.round(Math.hypot(mc.thePlayer.posX - mc.thePlayer.prevPosX, mc.thePlayer.posZ - mc.thePlayer.prevPosZ) * 200) / 10.0 + " blocks/s");
        if (client.enabled(Module.DIRECTION)) client.value(Module.DIRECTION, mc.thePlayer.getHorizontalFacing().getName().toUpperCase(Locale.ROOT));
        if (client.enabled(Module.BIOME)) client.value(Module.BIOME, mc.theWorld.getBiomeGenForCoords(mc.thePlayer.getPosition()).biomeName);
        if (client.enabled(Module.FOOD)) client.value(Module.FOOD, "Food " + mc.thePlayer.getFoodStats().getFoodLevel() + " / Sat " + Math.round(mc.thePlayer.getFoodStats().getSaturationLevel()));
        if (client.enabled(Module.WORLD_TIME)) { long time = (mc.theWorld.getWorldTime() + 6000) % 24000; client.value(Module.WORLD_TIME, time / 1000 + ":" + (time % 1000 * 60 / 1000 < 10 ? "0" : "") + time % 1000 * 60 / 1000); }
        if (client.enabled(Module.SERVER)) client.value(Module.SERVER, mc.getCurrentServerData() == null ? "Singleplayer" : mc.getCurrentServerData().serverIP);
        if (client.enabled(Module.HELD_ITEM)) {
            ItemStack item = mc.thePlayer.getHeldItem();
            client.value(Module.HELD_ITEM, item == null ? "Empty hand" : item.getDisplayName() + (item.isItemStackDamageable() ? " " + (item.getMaxDamage() - item.getItemDamage()) : " x" + item.stackSize));
        }
        if (client.enabled(Module.ARMOR)) {
        StringBuilder armor = new StringBuilder("Armor");
        for (ItemStack item : mc.thePlayer.inventory.armorInventory) if (item != null) armor.append(" ").append(item.getMaxDamage() - item.getItemDamage());
        client.value(Module.ARMOR, armor.toString());
        }
        if (client.enabled(Module.EFFECTS)) {
        StringBuilder effects = new StringBuilder();
        for (Object value : mc.thePlayer.getActivePotionEffects()) {
            PotionEffect effect = (PotionEffect) value;
            if (effects.length() > 0) effects.append(" / ");
            effects.append(net.minecraft.client.resources.I18n.format(effect.getEffectName())).append(" ").append(effect.getDuration() / 20).append("s");
        }
        client.value(Module.EFFECTS, effects.length() == 0 ? "No effects" : effects.toString());
        }
    }
    @SubscribeEvent public void fov(FOVUpdateEvent event) {
        event.newfov /= client.zoomFactor(zoom.isKeyDown() && mc.currentScreen == null);
    }
    @SubscribeEvent public void mouse(net.minecraftforge.client.event.MouseEvent event) {
        if (event.buttonstate && mc.currentScreen == null && (event.button == 0 || event.button == 1)) client.click(event.button == 1);
    }
    private final CairnClient.Canvas canvas = new CairnClient.Canvas() {
        public void rect(int x, int y, int width, int height, int color) { Gui.drawRect(x, y, x + width, y + height, color); }
        public void text(String text, int x, int y, int color) { mc.fontRendererObj.drawString(text, x, y, color, false); }
        public void textScaled(String text, int x, int y, int color, float scale, boolean shadow) {
            net.minecraft.client.renderer.GlStateManager.pushMatrix();
            net.minecraft.client.renderer.GlStateManager.translate(x, y, 0);
            net.minecraft.client.renderer.GlStateManager.scale(scale, scale, 1);
            mc.fontRendererObj.drawString(text, 0, 0, color, shadow);
            net.minecraft.client.renderer.GlStateManager.popMatrix();
        }
        public int textWidth(String text) { return mc.fontRendererObj.getStringWidth(text); }
    };
    @SubscribeEvent public void render(RenderGameOverlayEvent.Post event) {
        if (event.type != RenderGameOverlayEvent.ElementType.ALL || mc.thePlayer == null || mc.gameSettings.hideGUI || mc.currentScreen instanceof Menu) return;
        client.renderHud(canvas, event.resolution.getScaledWidth(), event.resolution.getScaledHeight());
    }
    @SubscribeEvent public void crosshair(RenderGameOverlayEvent.Pre event) {
        if (event.type == RenderGameOverlayEvent.ElementType.CROSSHAIRS && client.enabled(Module.CROSSHAIR) && client.setting(Module.CROSSHAIR, CairnClient.Setting.CROSS_HIDE_VANILLA) == 1) event.setCanceled(true);
    }
    private final class Menu extends GuiScreen {
        @Override public void drawScreen(int mouseX, int mouseY, float delta) { client.renderMenu(canvas, width, height, mouseX, mouseY); }
        @Override protected void mouseClicked(int x, int y, int button) throws IOException { if (button == 0) client.mouseDown(width, height, x, y); }
        @Override protected void mouseClickMove(int x, int y, int button, long elapsed) { if (button == 0) client.drag(width, height, x, y); }
        @Override protected void mouseReleased(int x, int y, int button) { client.mouseUp(); }
        @Override public void handleMouseInput() throws IOException { super.handleMouseInput(); int wheel = Mouse.getEventDWheel(); if (wheel != 0) client.wheel(wheel < 0 ? 1 : -1); }
        @Override protected void keyTyped(char character, int key) throws IOException {
            if (key == Keyboard.KEY_ESCAPE || key == Keyboard.KEY_RSHIFT) {
                if (!client.escape()) mc.displayGuiScreen(null);
            } else if (client.typing()) {
                if (key == Keyboard.KEY_BACK) client.backspace();
                else if (key == Keyboard.KEY_RETURN) client.key(0, true);
                else client.type(character);
            } else if (key == Keyboard.KEY_TAB) client.tab();
            else if (client.editing()) {
                if (key == Keyboard.KEY_G) client.snap();
                else if (key == Keyboard.KEY_H) client.toggleEditorTools();
                else if (key == Keyboard.KEY_RETURN) client.editorSettings();
                else if (key == Keyboard.KEY_EQUALS || key == Keyboard.KEY_ADD) client.editorKey(0, 0, 1);
                else if (key == Keyboard.KEY_MINUS || key == Keyboard.KEY_SUBTRACT) client.editorKey(0, 0, -1);
                else if (key == Keyboard.KEY_RIGHT) client.editorKey(1, 0, 0);
                else if (key == Keyboard.KEY_LEFT) client.editorKey(-1, 0, 0);
                else if (key == Keyboard.KEY_UP) client.editorKey(0, -1, 0);
                else if (key == Keyboard.KEY_DOWN) client.editorKey(0, 1, 0);
            } else if (key == Keyboard.KEY_E) client.enterEditor();
            else if (key == Keyboard.KEY_F && isCtrlKeyDown()) client.focusSearch();
            else if (key == Keyboard.KEY_RETURN || key == Keyboard.KEY_SPACE) client.key(0, true);
            else if (key == Keyboard.KEY_RIGHT || key == Keyboard.KEY_DOWN) client.key(1, false);
            else if (key == Keyboard.KEY_LEFT || key == Keyboard.KEY_UP) client.key(-1, false);
        }
        @Override public void onGuiClosed() { client.leaveEditor(); }
        @Override public boolean doesGuiPauseGame() { return false; }
    }
}

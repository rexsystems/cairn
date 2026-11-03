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
    private final java.util.Map<String, net.minecraft.util.ResourceLocation> textures = new java.util.HashMap<String, net.minecraft.util.ResourceLocation>();
    // Mojang capes we replaced, restored when a player unequips their Cairn cape.
    private final java.util.Map<net.minecraft.client.network.NetworkPlayerInfo, net.minecraft.util.ResourceLocation> mojangCapes = new java.util.WeakHashMap<net.minecraft.client.network.NetworkPlayerInfo, net.minecraft.util.ResourceLocation>();
    private java.lang.reflect.Field capeField;

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
        if (event.gui == mc.currentScreen && mc.currentScreen instanceof net.minecraft.client.gui.GuiMainMenu && Keyboard.getEventKeyState() && Keyboard.getEventKey() == menu.getKeyCode()) mc.displayGuiScreen(new Menu());
    }
    @SubscribeEvent public void tick(TickEvent.ClientTickEvent event) {
        if (event.phase != TickEvent.Phase.END) return;
        if (mc.currentScreen != null && mc.currentScreen.getClass() == net.minecraft.client.gui.GuiMainMenu.class) mc.displayGuiScreen(new Title());
        client.cosmetics.account(mc.getSession().getUsername(), mc.getSession().getPlayerID(), mc.getSession().getToken());
        client.cosmetics.tick();
        client.friends.activity(net.minecraftforge.fml.common.Loader.MC_VERSION, mc.theWorld != null, mc.theWorld == null || mc.getCurrentServerData() == null ? null : mc.getCurrentServerData().serverIP);
        client.friends.joinHandler(mc.theWorld == null ? address -> {
            if (mc.theWorld != null) return;
            mc.displayGuiScreen(new net.minecraft.client.multiplayer.GuiConnecting(new net.minecraft.client.gui.GuiMainMenu(), mc, FriendConnection.server(address)));
        } : null);
        if (mc.thePlayer == null) return;
        mc.gameSettings.smoothCamera = client.smoothCamera(zooming(), mc.gameSettings.smoothCamera);
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
        if (client.enabled(Module.BLOCK_INDICATOR)) {
            if (mc.objectMouseOver != null && mc.objectMouseOver.typeOfHit == net.minecraft.util.MovingObjectPosition.MovingObjectType.BLOCK) {
                net.minecraft.block.state.IBlockState state = mc.theWorld.getBlockState(mc.objectMouseOver.getBlockPos());
                net.minecraft.block.Block block = state.getBlock(); net.minecraft.item.Item item = net.minecraft.item.Item.getItemFromBlock(block);
                ItemStack stack = item == null ? null : new ItemStack(item, 1, block.getMetaFromState(state));
                client.blockTarget(stack, stack == null ? block.getLocalizedName() : stack.getDisplayName());
            } else client.blockTarget(null, null);
        }
        if (client.enabled(Module.REACH)) {
            if (mc.objectMouseOver != null && mc.objectMouseOver.typeOfHit != net.minecraft.util.MovingObjectPosition.MovingObjectType.MISS)
                client.reach(mc.thePlayer.getPositionEyes(1).distanceTo(mc.objectMouseOver.hitVec));
            else client.reach(-1);
        }
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
        java.util.List<CairnClient.HudRow> rows = new java.util.ArrayList<CairnClient.HudRow>();
        for (ItemStack item : mc.thePlayer.inventory.armorInventory) if (item != null) { armor.append(" ").append(item.getMaxDamage() - item.getItemDamage()); rows.add(new CairnClient.HudRow(item.copy(), String.valueOf(item.getMaxDamage() - item.getItemDamage()))); }
        client.value(Module.ARMOR, armor.toString());
        client.rows(Module.ARMOR, rows);
        }
        if (client.enabled(Module.EFFECTS)) {
        java.util.List<CairnClient.HudRow> rows = new java.util.ArrayList<CairnClient.HudRow>();
        StringBuilder effects = new StringBuilder();
        for (Object value : mc.thePlayer.getActivePotionEffects()) {
            PotionEffect effect = (PotionEffect) value;
            if (effects.length() > 0) effects.append(" / ");
            effects.append(net.minecraft.client.resources.I18n.format(effect.getEffectName())).append(" ").append(effect.getDuration() / 20).append("s");
            rows.add(new CairnClient.HudRow(effect, net.minecraft.client.resources.I18n.format(effect.getEffectName()) + " " + (effect.getAmplifier() + 1) + " / " + effect.getDuration() / 20 + "s"));
        }
        client.value(Module.EFFECTS, effects.length() == 0 ? "No effects" : effects.toString());
        client.rows(Module.EFFECTS, rows);
        }
    }
    /** Uploads cosmetics PNGs lazily on the render thread; a broken image is remembered so it isn't decoded every frame. */
    private net.minecraft.util.ResourceLocation texture(String key) {
        if ("preview/world".equals(key)) return new net.minecraft.util.ResourceLocation("textures/gui/title/background/panorama_0.png");
        if (key == null) return null;
        if (textures.containsKey(key)) return textures.get(key);
        byte[] png = client.cosmetics.texture(key);
        if (png == null) return null;
        net.minecraft.util.ResourceLocation id = new net.minecraft.util.ResourceLocation("cairn_client", "cosmetics/" + key);
        try {
            java.awt.image.BufferedImage image = javax.imageio.ImageIO.read(new java.io.ByteArrayInputStream(png));
            if (image == null) throw new IOException("not a PNG");
            mc.getTextureManager().loadTexture(id, new net.minecraft.client.renderer.texture.DynamicTexture(image));
        } catch (IOException | RuntimeException e) {
            System.err.println("[Cairn Client] Could not upload " + key + ": " + e);
            id = null;
        }
        textures.put(key, id);
        return id;
    }
    private app.cairn.client.Cosmetics.Look look(net.minecraft.entity.Entity entity) {
        return entity instanceof net.minecraft.client.entity.AbstractClientPlayer ? client.cosmetics.look(entity.getUniqueID().toString()) : null;
    }
    /** 1.8 has no cape getter, so LayerCape reads NetworkPlayerInfo; swap that location on the render path (animated frames update each draw). */
    @SubscribeEvent public void cape(net.minecraftforge.client.event.RenderPlayerEvent.Pre event) {
        if (mc.getNetHandler() == null) return;
        net.minecraft.client.network.NetworkPlayerInfo info = mc.getNetHandler().getPlayerInfo(event.entityPlayer.getUniqueID());
        if (info == null) return;
        try {
            if (capeField == null) capeField = net.minecraftforge.fml.relauncher.ReflectionHelper.findField(net.minecraft.client.network.NetworkPlayerInfo.class, "locationCape", "field_178862_f");
            app.cairn.client.Cosmetics.Look look = look(event.entityPlayer);
            net.minecraft.util.ResourceLocation cape = look == null ? null : texture(client.cosmetics.capeTexture(look));
            net.minecraft.util.ResourceLocation current = (net.minecraft.util.ResourceLocation) capeField.get(info);
            boolean ours = current != null && "cairn_client".equals(current.getResourceDomain());
            if (cape != null) {
                if (!ours) mojangCapes.put(info, current);
                if (cape != current) capeField.set(info, cape);
            } else if (ours) capeField.set(info, mojangCapes.remove(info));
        } catch (ReflectiveOperationException | RuntimeException e) {
            capeField = null;
        }
    }
    /** Mirrors Render.renderLivingLabel so the logo badge sits just left of the vanilla name. */
    @SubscribeEvent public void badge(net.minecraftforge.client.event.RenderLivingEvent.Specials.Post event) {
        net.minecraft.entity.EntityLivingBase entity = event.entity;
        net.minecraft.client.renderer.entity.RenderManager manager = mc.getRenderManager();
        if (entity == manager.livingPlayer || entity.isInvisibleToPlayer(mc.thePlayer) || entity.riddenByEntity != null || !Minecraft.isGuiEnabled()) return;
        if (entity.getDistanceSqToEntity(manager.livingPlayer) > (entity.isSneaking() ? 32 * 32 : 64 * 64)) return;
        net.minecraft.util.ResourceLocation icon = texture(client.cosmetics.badgeTexture(look(entity)));
        if (icon == null) return;
        String name = entity.getDisplayName().getFormattedText();
        float x0 = -mc.fontRendererObj.getStringWidth(name) / 2f - 11, x1 = x0 + 9, y0 = ("deadmau5".equals(name) ? -10 : 0) - .5f, y1 = y0 + 9;
        float scale = 0.016666668F * 1.6F;
        float height = entity.height + 0.5F;
        // Match RenderPlayer.renderOffsetLivingLabel: below-name score lifts the real name, so lift the badge with it.
        if (entity instanceof net.minecraft.client.entity.AbstractClientPlayer && entity.getDistanceSqToEntity(manager.livingPlayer) < 100 * 100) {
            net.minecraft.scoreboard.Scoreboard board = ((net.minecraft.client.entity.AbstractClientPlayer) entity).getWorldScoreboard();
            if (board != null && board.getObjectiveInDisplaySlot(2) != null) height += mc.fontRendererObj.FONT_HEIGHT * 1.15F * scale;
        }
        net.minecraft.client.renderer.GlStateManager.pushMatrix();
        net.minecraft.client.renderer.GlStateManager.translate((float) event.x, (float) event.y + height, (float) event.z);
        net.minecraft.client.renderer.GlStateManager.rotate(-manager.playerViewY, 0, 1, 0);
        net.minecraft.client.renderer.GlStateManager.rotate(manager.playerViewX, 1, 0, 0);
        net.minecraft.client.renderer.GlStateManager.scale(-scale, -scale, scale);
        net.minecraft.client.renderer.GlStateManager.disableLighting();
        net.minecraft.client.renderer.GlStateManager.enableBlend();
        net.minecraft.client.renderer.GlStateManager.tryBlendFuncSeparate(770, 771, 1, 0);
        net.minecraft.client.renderer.GlStateManager.color(1, 1, 1, 1);
        mc.getTextureManager().bindTexture(icon);
        // Immediate mode: WorldRenderer's vertex API differs between 1.8 and 1.8.8+.
        org.lwjgl.opengl.GL11.glBegin(org.lwjgl.opengl.GL11.GL_QUADS);
        org.lwjgl.opengl.GL11.glTexCoord2f(0, 0); org.lwjgl.opengl.GL11.glVertex3f(x0, y0, 0);
        org.lwjgl.opengl.GL11.glTexCoord2f(0, 1); org.lwjgl.opengl.GL11.glVertex3f(x0, y1, 0);
        org.lwjgl.opengl.GL11.glTexCoord2f(1, 1); org.lwjgl.opengl.GL11.glVertex3f(x1, y1, 0);
        org.lwjgl.opengl.GL11.glTexCoord2f(1, 0); org.lwjgl.opengl.GL11.glVertex3f(x1, y0, 0);
        org.lwjgl.opengl.GL11.glEnd();
        net.minecraft.client.renderer.GlStateManager.enableLighting();
        net.minecraft.client.renderer.GlStateManager.disableBlend();
        net.minecraft.client.renderer.GlStateManager.popMatrix();
    }
    @SubscribeEvent public void fov(FOVUpdateEvent event) {
        event.newfov /= client.zoomFactor(zooming());
    }
    private boolean zooming() { return zoom.isKeyDown() && mc.currentScreen == null && mc.thePlayer != null; }
    @SubscribeEvent public void mouse(net.minecraftforge.client.event.MouseEvent event) {
        if (event.buttonstate && mc.currentScreen == null && (event.button == 0 || event.button == 1)) client.click(event.button == 1);
        // While zoomed, the wheel changes zoom instead of the hotbar slot.
        if (event.dwheel != 0 && client.zoomScroll(zooming(), event.dwheel)) event.setCanceled(true);
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
        public void icon(Object icon, int x, int y, int size) {
            net.minecraft.client.renderer.GlStateManager.pushMatrix();
            net.minecraft.client.renderer.GlStateManager.translate(x, y, 0); net.minecraft.client.renderer.GlStateManager.scale(size / 16f, size / 16f, 1);
            if (icon instanceof ItemStack) {
                net.minecraft.client.renderer.RenderHelper.enableGUIStandardItemLighting();
                mc.getRenderItem().renderItemAndEffectIntoGUI((ItemStack) icon, 0, 0);
                net.minecraft.client.renderer.RenderHelper.disableStandardItemLighting();
            } else if (icon instanceof PotionEffect) {
                net.minecraft.potion.Potion potion = net.minecraft.potion.Potion.potionTypes[((PotionEffect) icon).getPotionID()];
                if (potion != null && potion.hasStatusIcon()) {
                    int index = potion.getStatusIconIndex();
                    mc.getTextureManager().bindTexture(new net.minecraft.util.ResourceLocation("textures/gui/container/inventory.png"));
                    net.minecraft.client.renderer.GlStateManager.color(1, 1, 1, 1);
                    Gui.drawScaledCustomSizeModalRect(0, 0, index % 8 * 18, 198 + index / 8 * 18, 18, 18, 16, 16, 256, 256);
                }
            }
            net.minecraft.client.renderer.GlStateManager.popMatrix();
        }
        public void image(String key, int x, int y, int width, int height, float u0, float v0, float u1, float v1) {
            net.minecraft.util.ResourceLocation id = texture(key);
            if (id == null) return;
            // This blit takes whole texels, so normalized UVs become a virtual texture size.
            int textureWidth = Math.round(width / (u1 - u0)), textureHeight = Math.round(height / (v1 - v0));
            net.minecraft.client.renderer.GlStateManager.color(1, 1, 1, 1);
            net.minecraft.client.renderer.GlStateManager.enableBlend();
            mc.getTextureManager().bindTexture(id);
            Gui.drawModalRectWithCustomSizedTexture(x, y, u0 * textureWidth, v0 * textureHeight, width, height, textureWidth, textureHeight);
        }
    };
    @SubscribeEvent public void render(RenderGameOverlayEvent.Post event) {
        if (event.type != RenderGameOverlayEvent.ElementType.ALL || mc.thePlayer == null || mc.gameSettings.hideGUI || mc.currentScreen instanceof Menu) return;
        client.renderHud(canvas, event.resolution.getScaledWidth(), event.resolution.getScaledHeight());
    }
    @SubscribeEvent public void crosshair(RenderGameOverlayEvent.Pre event) {
        if (event.type == RenderGameOverlayEvent.ElementType.CROSSHAIRS && client.enabled(Module.CROSSHAIR) && client.setting(Module.CROSSHAIR, CairnClient.Setting.CROSS_HIDE_VANILLA) == 1) event.setCanceled(true);
    }
    private final class Title extends net.minecraft.client.gui.GuiMainMenu {
        private app.cairn.client.CairnTitle view;
        @Override public void initGui() {
            super.initGui(); view = new app.cairn.client.CairnTitle();
            nativeAction(1, "Singleplayer"); nativeAction(2, "Multiplayer");
            view.add("Cairn Settings", () -> { mc.displayGuiScreen(new Menu()); client.openModules(); }, true);
            view.add("Cosmetics", () -> { mc.displayGuiScreen(new Menu()); client.openCosmetics(); }, true);
            view.add("Friends", () -> { mc.displayGuiScreen(new Menu()); client.openFriends(); }, true);
            nativeAction(0, "Minecraft settings"); nativeAction(4, "Quit game");
            client.cosmetics.badgeTexture("classic");
        }
        private void nativeAction(int id, String label) {
            for (Object value : buttonList) {
                final net.minecraft.client.gui.GuiButton button = (net.minecraft.client.gui.GuiButton) value;
                if (button.id == id) view.add(label, () -> { try { super.actionPerformed(button); } catch (IOException e) { throw new RuntimeException(e); } }, button.enabled);
            }
        }
        @Override public void drawScreen(int x, int y, float delta) {
            net.minecraft.client.renderer.GlStateManager.color(1, 1, 1, 1);
            mc.getTextureManager().bindTexture(new net.minecraft.util.ResourceLocation("textures/gui/title/background/panorama_0.png"));
            Gui.drawScaledCustomSizeModalRect(0, 0, 0, 0, 256, 256, width, height, 256, 256);
            view.render(canvas, width, height, x, y);
        }
        @Override protected void mouseClicked(int x, int y, int button) { if (button == 0) view.click(x, y); }
        @Override protected void keyTyped(char c, int key) {
            if (key == menu.getKeyCode()) { mc.displayGuiScreen(new Menu()); return; }
            if (key == Keyboard.KEY_TAB || key == Keyboard.KEY_DOWN || key == Keyboard.KEY_UP) view.key(false, key == Keyboard.KEY_UP || isShiftKeyDown());
            else if (key == Keyboard.KEY_RETURN || key == Keyboard.KEY_SPACE) view.key(true, false);
        }
    }
    private final class Menu extends GuiScreen {
        Menu() { client.enterEditor(); }
        @Override public void updateScreen() { if (client.consumeMenuClose()) mc.displayGuiScreen(null); }
        @Override public void drawScreen(int mouseX, int mouseY, float delta) { client.renderMenu(canvas, width, height, mouseX, mouseY); }
        @Override protected void mouseClicked(int x, int y, int button) throws IOException { if (button == 0) client.mouseDown(width, height, x, y); }
        @Override protected void mouseClickMove(int x, int y, int button, long elapsed) { if (button == 0) client.drag(width, height, x, y); }
        @Override protected void mouseReleased(int x, int y, int button) { client.mouseUp(); }
        @Override public void handleMouseInput() throws IOException { super.handleMouseInput(); int wheel = Mouse.getEventDWheel(); if (wheel != 0) client.wheel(wheel < 0 ? 1 : -1); }
        @Override protected void keyTyped(char character, int key) throws IOException {
            if (key == menu.getKeyCode() && key != Keyboard.KEY_TAB && (key == Keyboard.KEY_RSHIFT || !client.typing())) { mc.displayGuiScreen(null); return; }
            if (key == Keyboard.KEY_ESCAPE) {
                if (!client.escape()) mc.displayGuiScreen(null);
            } else if (key == Keyboard.KEY_TAB) client.tab();
            else if (client.typing()) {
                if (key == Keyboard.KEY_BACK) client.backspace();
                else if (key == Keyboard.KEY_RETURN) client.key(0, true);
                else client.type(character);
            } else if (client.editing()) {
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

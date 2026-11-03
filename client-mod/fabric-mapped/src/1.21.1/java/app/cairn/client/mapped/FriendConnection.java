package app.cairn.client.mapped;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.ConnectScreen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.client.multiplayer.resolver.ServerAddress;

final class FriendConnection {
    static void join(String address) {
        Minecraft mc = Minecraft.getInstance();
        if (mc.level != null) return;
        ConnectScreen.startConnecting(new TitleScreen(), mc, ServerAddress.parseString(address), new ServerData(address, address, ServerData.Type.OTHER), false, null);
    }
}

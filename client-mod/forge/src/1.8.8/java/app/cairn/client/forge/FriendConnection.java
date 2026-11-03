package app.cairn.client.forge;

final class FriendConnection {
    static net.minecraft.client.multiplayer.ServerData server(String address) {
        return new net.minecraft.client.multiplayer.ServerData(address, address, false);
    }
}

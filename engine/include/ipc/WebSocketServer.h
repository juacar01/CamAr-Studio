#pragma once

#include "core/Types.h"
#include <string>
#include <functional>
#include <atomic>
#include <thread>
#include <vector>
#include <mutex>
#include <winsock2.h>
#include <ws2tcpip.h>

#pragma comment(lib, "ws2_32.lib")

namespace CamAr {

using MessageCallback = std::function<void(const std::string& message)>;

class WebSocketServer {
public:
    WebSocketServer();
    ~WebSocketServer();

    bool Start(int port, MessageCallback onMessage);
    void Stop();

    // Envía mensaje de texto JSON a todos los clientes conectados
    void BroadcastText(const std::string& jsonText);

    // Envía buffer binario (para preview de video si se requiere)
    void BroadcastBinary(const uint8_t* data, size_t size);

    bool IsRunning() const { return m_running; }
    size_t GetClientCount() const;

private:
    void ListenLoop();
    void HandleClient(SOCKET clientSocket);
    std::string PerformHandshake(const std::string& requestHeaders);
    std::string EncodeFrame(const std::string& payload, bool isBinary = false);

    int m_port = 8765;
    std::atomic<bool> m_running{false};
    SOCKET m_listenSocket = INVALID_SOCKET;
    std::thread m_listenThread;
    MessageCallback m_onMessage;

    mutable std::mutex m_clientsMutex;
    std::vector<SOCKET> m_clients;
};

} // namespace CamAr

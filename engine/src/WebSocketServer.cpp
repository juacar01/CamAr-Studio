#include "ipc/WebSocketServer.h"
#include <iostream>
#include <sstream>
#include <wincrypt.h>

#pragma comment(lib, "crypt32.lib")

namespace CamAr {

static std::string ComputeWebSocketAccept(const std::string& clientKey) {
    std::string concatenated = clientKey + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    HCRYPTPROV hProv = 0;
    HCRYPTHASH hHash = 0;
    std::string result = "";

    if (CryptAcquireContext(&hProv, NULL, NULL, PROV_RSA_FULL, CRYPT_VERIFYCONTEXT)) {
        if (CryptCreateHash(hProv, CALG_SHA1, 0, 0, &hHash)) {
            if (CryptHashData(hHash, (const BYTE*)concatenated.c_str(), (DWORD)concatenated.length(), 0)) {
                BYTE hash[20];
                DWORD hashLen = 20;
                if (CryptGetHashParam(hHash, HP_HASHVAL, hash, &hashLen, 0)) {
                    DWORD base64Len = 0;
                    CryptBinaryToStringA(hash, hashLen, CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, NULL, &base64Len);
                    std::vector<char> base64(base64Len);
                    CryptBinaryToStringA(hash, hashLen, CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, base64.data(), &base64Len);
                    result = std::string(base64.data());
                }
            }
            CryptDestroyHash(hHash);
        }
        CryptReleaseContext(hProv, 0);
    }
    return result;
}

WebSocketServer::WebSocketServer() {
    WSADATA wsaData;
    WSAStartup(MAKEWORD(2, 2), &wsaData);
}

WebSocketServer::~WebSocketServer() {
    Stop();
    WSACleanup();
}

bool WebSocketServer::Start(int port, MessageCallback onMessage) {
    Stop();

    m_port = port;
    m_onMessage = onMessage;

    m_listenSocket = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (m_listenSocket == INVALID_SOCKET) {
        return false;
    }

    // Permitir reutilizar dirección
    BOOL opt = TRUE;
    setsockopt(m_listenSocket, SOL_SOCKET, SO_REUSEADDR, (const char*)&opt, sizeof(opt));

    sockaddr_in serverAddr = {};
    serverAddr.sin_family = AF_INET;
    serverAddr.sin_addr.s_addr = htonl(INADDR_LOOPBACK); // Solo localhost para seguridad
    serverAddr.sin_port = htons(m_port);

    if (bind(m_listenSocket, (sockaddr*)&serverAddr, sizeof(serverAddr)) == SOCKET_ERROR) {
        closesocket(m_listenSocket);
        m_listenSocket = INVALID_SOCKET;
        return false;
    }

    if (listen(m_listenSocket, SOMAXCONN) == SOCKET_ERROR) {
        closesocket(m_listenSocket);
        m_listenSocket = INVALID_SOCKET;
        return false;
    }

    m_running = true;
    m_listenThread = std::thread(&WebSocketServer::ListenLoop, this);
    std::cout << "[WebSocketServer] IPC Server escuchando en ws://127.0.0.1:" << m_port << std::endl;
    return true;
}

void WebSocketServer::Stop() {
    m_running = false;
    if (m_listenSocket != INVALID_SOCKET) {
        closesocket(m_listenSocket);
        m_listenSocket = INVALID_SOCKET;
    }

    {
        std::lock_guard<std::mutex> lock(m_clientsMutex);
        for (SOCKET s : m_clients) {
            closesocket(s);
        }
        m_clients.clear();
    }

    if (m_listenThread.joinable()) {
        m_listenThread.join();
    }
}

size_t WebSocketServer::GetClientCount() const {
    std::lock_guard<std::mutex> lock(m_clientsMutex);
    return m_clients.size();
}

void WebSocketServer::ListenLoop() {
    while (m_running) {
        sockaddr_in clientAddr;
        int clientLen = sizeof(clientAddr);
        SOCKET client = accept(m_listenSocket, (sockaddr*)&clientAddr, &clientLen);

        if (client == INVALID_SOCKET) {
            if (!m_running) break;
            continue;
        }

        // Manejar cada cliente en un hilo separado
        std::thread([this, client]() {
            this->HandleClient(client);
        }).detach();
    }
}

void WebSocketServer::HandleClient(SOCKET clientSocket) {
    char buffer[4096];
    int bytesRead = recv(clientSocket, buffer, sizeof(buffer) - 1, 0);
    if (bytesRead <= 0) {
        closesocket(clientSocket);
        return;
    }
    buffer[bytesRead] = '\0';
    std::string request(buffer, bytesRead);

    // Buscar Sec-WebSocket-Key
    std::string keyHeader = "Sec-WebSocket-Key: ";
    size_t keyPos = request.find(keyHeader);
    if (keyPos == std::string::npos) {
        closesocket(clientSocket);
        return;
    }

    size_t keyEnd = request.find("\r\n", keyPos);
    std::string clientKey = request.substr(keyPos + keyHeader.length(), keyEnd - (keyPos + keyHeader.length()));

    std::string acceptVal = ComputeWebSocketAccept(clientKey);
    std::ostringstream response;
    response << "HTTP/1.1 101 Switching Protocols\r\n"
             << "Upgrade: websocket\r\n"
             << "Connection: Upgrade\r\n"
             << "Sec-WebSocket-Accept: " << acceptVal << "\r\n\r\n";

    std::string respStr = response.str();
    send(clientSocket, respStr.c_str(), (int)respStr.length(), 0);

    {
        std::lock_guard<std::mutex> lock(m_clientsMutex);
        m_clients.push_back(clientSocket);
    }
    std::cout << "[WebSocketServer] Cliente UI conectado exitosamente." << std::endl;

    // Bucle de lectura de frames WebSocket RFC 6455
    while (m_running) {
        uint8_t header[2];
        int r = recv(clientSocket, (char*)header, 2, 0);
        if (r <= 0) break;

        uint8_t opcode = header[0] & 0x0F;
        bool masked = (header[1] & 0x80) != 0;
        uint64_t payloadLen = header[1] & 0x7F;

        if (opcode == 0x8) {
            // Close frame
            break;
        }

        if (payloadLen == 126) {
            uint16_t extLen = 0;
            recv(clientSocket, (char*)&extLen, 2, 0);
            payloadLen = ntohs(extLen);
        } else if (payloadLen == 127) {
            uint64_t extLen = 0;
            recv(clientSocket, (char*)&extLen, 8, 0);
            // big-endian 64
            payloadLen = extLen; // simplificado
        }

        uint8_t maskKey[4] = {0};
        if (masked) {
            recv(clientSocket, (char*)maskKey, 4, 0);
        }

        std::vector<char> payload(payloadLen);
        size_t totalReceived = 0;
        while (totalReceived < payloadLen) {
            int received = recv(clientSocket, payload.data() + totalReceived, (int)(payloadLen - totalReceived), 0);
            if (received <= 0) break;
            totalReceived += received;
        }

        if (masked) {
            for (size_t i = 0; i < payloadLen; ++i) {
                payload[i] ^= maskKey[i % 4];
            }
        }

        if (opcode == 0x1) { // Text frame
            std::string msg(payload.data(), payloadLen);
            if (m_onMessage) {
                m_onMessage(msg);
            }
        }
    }

    {
        std::lock_guard<std::mutex> lock(m_clientsMutex);
        auto it = std::find(m_clients.begin(), m_clients.end(), clientSocket);
        if (it != m_clients.end()) {
            m_clients.erase(it);
        }
    }
    closesocket(clientSocket);
    std::cout << "[WebSocketServer] Cliente UI desconectado." << std::endl;
}

std::string WebSocketServer::EncodeFrame(const std::string& payload, bool isBinary) {
    std::string frame;
    frame.push_back((char)(0x80 | (isBinary ? 0x02 : 0x01))); // FIN + opcode

    size_t len = payload.length();
    if (len <= 125) {
        frame.push_back((char)len);
    } else if (len <= 65535) {
        frame.push_back((char)126);
        frame.push_back((char)((len >> 8) & 0xFF));
        frame.push_back((char)(len & 0xFF));
    } else {
        frame.push_back((char)127);
        for (int i = 7; i >= 0; --i) {
            frame.push_back((char)((len >> (i * 8)) & 0xFF));
        }
    }

    frame.append(payload);
    return frame;
}

void WebSocketServer::BroadcastText(const std::string& jsonText) {
    std::string frame = EncodeFrame(jsonText, false);
    std::lock_guard<std::mutex> lock(m_clientsMutex);
    for (SOCKET s : m_clients) {
        send(s, frame.data(), (int)frame.size(), 0);
    }
}

void WebSocketServer::BroadcastBinary(const uint8_t* data, size_t size) {
    std::string payload((const char*)data, size);
    std::string frame = EncodeFrame(payload, true);
    std::lock_guard<std::mutex> lock(m_clientsMutex);
    for (SOCKET s : m_clients) {
        send(s, frame.data(), (int)frame.size(), 0);
    }
}

} // namespace CamAr

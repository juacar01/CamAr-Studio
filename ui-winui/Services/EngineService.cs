using System;
using System.Diagnostics;
using System.IO;
using System.Net.WebSockets;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using CamArStudio_WinUI.Models;

namespace CamArStudio_WinUI.Services
{
    public class EngineService
    {
        private readonly Uri _wsUri = new("ws://127.0.0.1:8765");
        private ClientWebSocket? _webSocket;
        private CancellationTokenSource? _cts;
        private bool _isConnected;
        private Process? _engineProcess;

        // Throttling for slider dragging
        private readonly Timer _debounceTimer;
        private StudioConfig? _pendingConfig;
        private readonly object _configLock = new();

        public event Action<byte[]>? FrameReceived;
        public event Action<EngineTelemetry>? TelemetryReceived;
        public event Action<DevicesPayload>? DevicesReceived;
        public event Action<StudioConfig>? PresetLoaded;
        public event Action<bool>? ConnectionStatusChanged;

        public bool IsConnected => _isConnected;

        public EngineService()
        {
            _debounceTimer = new Timer(OnDebounceTimerFired, null, Timeout.Infinite, Timeout.Infinite);
        }

        public void Start()
        {
            EnsureEngineRunning();
            _cts = new CancellationTokenSource();
            _ = ConnectionLoopAsync(_cts.Token);
        }

        public void Stop()
        {
            _cts?.Cancel();
            _webSocket?.Dispose();
            _webSocket = null;
            _debounceTimer.Dispose();
        }

        private void EnsureEngineRunning()
        {
            try
            {
                var existing = Process.GetProcessesByName("camar-engine");
                if (existing.Length > 0)
                {
                    return;
                }

                // Look for release executable first, then debug
                string baseDir = AppDomain.CurrentDomain.BaseDirectory;
                string[] candidatePaths = {
                    Path.Combine(baseDir, "..", "..", "..", "..", "..", "engine-rs", "target", "release", "camar-engine.exe"),
                    Path.Combine(baseDir, "..", "..", "..", "..", "..", "target", "release", "camar-engine.exe"),
                    Path.GetFullPath(Path.Combine(baseDir, @"..\..\..\..\..\engine-rs\target\release\camar-engine.exe")),
                    @"D:\proyectos\CamAr Studio\engine-rs\target\release\camar-engine.exe",
                    Path.Combine(baseDir, "camar-engine.exe")
                };

                string? foundPath = null;
                foreach (var p in candidatePaths)
                {
                    var full = Path.GetFullPath(p);
                    if (File.Exists(full))
                    {
                        foundPath = full;
                        break;
                    }
                }

                if (foundPath != null)
                {
                    string rootWorkspace = @"D:\proyectos\CamAr Studio";
                    var psi = new ProcessStartInfo
                    {
                        FileName = foundPath,
                        Arguments = "--server-only",
                        WorkingDirectory = Directory.Exists(rootWorkspace) ? rootWorkspace : Path.GetDirectoryName(foundPath)!,
                        UseShellExecute = false,
                        CreateNoWindow = true
                    };
                    _engineProcess = Process.Start(psi);
                }
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"[EngineService] Error starting engine: {ex.Message}");
            }
        }

        private async Task ConnectionLoopAsync(CancellationToken token)
        {
            while (!token.IsCancellationRequested)
            {
                try
                {
                    _webSocket = new ClientWebSocket();
                    await _webSocket.ConnectAsync(_wsUri, token);

                    _isConnected = true;
                    ConnectionStatusChanged?.Invoke(true);

                    await ReceiveLoopAsync(_webSocket, token);
                }
                catch
                {
                    _isConnected = false;
                    ConnectionStatusChanged?.Invoke(false);
                }

                if (!token.IsCancellationRequested)
                {
                    await Task.Delay(1000, token);
                }
            }
        }

        private async Task ReceiveLoopAsync(ClientWebSocket ws, CancellationToken token)
        {
            var buffer = new byte[1024 * 64];
            using var ms = new MemoryStream();

            while (ws.State == WebSocketState.Open && !token.IsCancellationRequested)
            {
                WebSocketReceiveResult result;
                ms.SetLength(0);

                do
                {
                    result = await ws.ReceiveAsync(new ArraySegment<byte>(buffer), token);
                    if (result.MessageType == WebSocketMessageType.Close)
                    {
                        await ws.CloseAsync(WebSocketCloseStatus.NormalClosure, "Closing", token);
                        return;
                    }
                    ms.Write(buffer, 0, result.Count);
                } while (!result.EndOfMessage);

                if (result.MessageType == WebSocketMessageType.Binary)
                {
                    // Video frame payload (JPEG byte array)
                    var frameBytes = ms.ToArray();
                    FrameReceived?.Invoke(frameBytes);
                }
                else if (result.MessageType == WebSocketMessageType.Text)
                {
                    var jsonStr = Encoding.UTF8.GetString(ms.ToArray());
                    HandleTextMessage(jsonStr);
                }
            }
        }

        private void HandleTextMessage(string jsonStr)
        {
            try
            {
                using var doc = JsonDocument.Parse(jsonStr);
                if (doc.RootElement.TryGetProperty("type", out var typeProp))
                {
                    var type = typeProp.GetString();
                    if (type == "TELEMETRY")
                    {
                        var telemetry = JsonSerializer.Deserialize<EngineTelemetry>(jsonStr);
                        if (telemetry != null) TelemetryReceived?.Invoke(telemetry);
                    }
                    else if (type == "DEVICES_LIST")
                    {
                        var devices = JsonSerializer.Deserialize<DevicesPayload>(jsonStr);
                        if (devices != null) DevicesReceived?.Invoke(devices);
                    }
                    else if (type == "LOAD_PRESET")
                    {
                        if (doc.RootElement.TryGetProperty("config", out var configProp))
                        {
                            var config = JsonSerializer.Deserialize<StudioConfig>(configProp.GetRawText());
                            if (config != null) PresetLoaded?.Invoke(config);
                        }
                    }
                }
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"[EngineService] Error parsing text message: {ex.Message}");
            }
        }

        public void SendConfigUpdate(StudioConfig config)
        {
            lock (_configLock)
            {
                _pendingConfig = config;
                _debounceTimer.Change(16, Timeout.Infinite); // 60 Hz throttle
            }
        }

        public void SendConfig(StudioConfig config) => SendConfigUpdate(config);

        private void OnDebounceTimerFired(object? state)
        {
            StudioConfig? toSend;
            lock (_configLock)
            {
                toSend = _pendingConfig;
                _pendingConfig = null;
            }

            if (toSend != null)
            {
                var payload = new
                {
                    type = "UPDATE_CONFIG",
                    config = toSend
                };
                SendJsonAsync(payload);
            }
        }

        public void SelectCamera(string cameraId)
        {
            var payload = new
            {
                type = "SELECT_CAMERA",
                cameraId = cameraId
            };
            SendJsonAsync(payload);
        }

        public void SetVirtualCamera(bool active)
        {
            var payload = new
            {
                type = "SET_VIRTUAL_CAM",
                active = active
            };
            SendJsonAsync(payload);
        }

        public void ResetSettings()
        {
            var payload = new
            {
                type = "RESET_SETTINGS"
            };
            SendJsonAsync(payload);
        }

        private async void SendJsonAsync(object payload)
        {
            if (_webSocket != null && _webSocket.State == WebSocketState.Open)
            {
                try
                {
                    var json = JsonSerializer.Serialize(payload);
                    var bytes = Encoding.UTF8.GetBytes(json);
                    await _webSocket.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, CancellationToken.None);
                }
                catch (Exception ex)
                {
                    Debug.WriteLine($"[EngineService] Error sending JSON: {ex.Message}");
                }
            }
        }
    }
}

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Storage.Streams;
using Windows.Graphics.Imaging;
using CamArStudio_WinUI.Models;
using CamArStudio_WinUI.Services;

namespace CamArStudio_WinUI
{
    public sealed partial class MainPage : Page
    {
        private readonly EngineService _engineService;
        private StudioConfig _currentConfig = new();
        private DevicesPayload? _latestDevices;
        private bool _isInitialized;
        private bool _isUpdatingUiFromBackend;
        private bool _isRenderingFrame;
        private SoftwareBitmapSource? _softwareBitmapSource;
        private bool _isAudioMuted;

        public MainPage()
        {
            try
            {
                _engineService = new EngineService();
                _engineService.ConnectionStatusChanged += OnConnectionStatusChanged;
                _engineService.FrameReceived += OnFrameReceived;
                _engineService.TelemetryReceived += OnTelemetryReceived;
                _engineService.DevicesReceived += OnDevicesReceived;
                _engineService.PresetLoaded += OnPresetLoaded;

                this.InitializeComponent();

                this.Loaded += MainPage_Loaded;
                this.Unloaded += MainPage_Unloaded;
            }
            catch (Exception ex)
            {
                try { File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] MainPage Exception: {ex.Message}\n{ex.StackTrace}\n"); } catch { }
                throw;
            }
        }

        private void MainPage_Loaded(object sender, RoutedEventArgs e)
        {
            _softwareBitmapSource = new SoftwareBitmapSource();
            VideoPreviewImage.Source = _softwareBitmapSource;

            _isInitialized = true;
            UpdateValueLabels();
            _engineService.Start();
        }

        private void MainPage_Unloaded(object sender, RoutedEventArgs e)
        {
            _engineService.Stop();
            _softwareBitmapSource?.Dispose();
            _softwareBitmapSource = null;
        }

        // =====================================================================
        // 1. ENGINE CONNECTION & FRAME RENDERING
        // =====================================================================
        private void OnConnectionStatusChanged(bool connected)
        {
            DispatcherQueue.TryEnqueue(() =>
            {
                try
                {
                    if (connected)
                    {
                        StatusDot.Fill = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 16, 185, 129));
                        EngineStatusBadge.Background = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 20, 44, 29));
                        StatusText.Text = "Conectado";
                        StatusText.Foreground = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 52, 211, 153));
                        NoSignalOverlay.Visibility = Visibility.Collapsed;
                    }
                    else
                    {
                        StatusDot.Fill = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 239, 68, 68));
                        EngineStatusBadge.Background = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 45, 20, 20));
                        StatusText.Text = "Reconectando...";
                        StatusText.Foreground = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 252, 165, 165));
                        NoSignalOverlay.Visibility = Visibility.Visible;
                    }
                }
                catch { }
            });
        }

        private void OnFrameReceived(byte[] jpegBytes)
        {
            if (_isRenderingFrame || jpegBytes == null || jpegBytes.Length == 0) return;
            _isRenderingFrame = true;

            DispatcherQueue.TryEnqueue(async () =>
            {
                try
                {
                    using var stream = new InMemoryRandomAccessStream();
                    await stream.WriteAsync(jpegBytes.AsBuffer());
                    stream.Seek(0);

                    var decoder = await BitmapDecoder.CreateAsync(stream);
                    using var rawBitmap = await decoder.GetSoftwareBitmapAsync();
                    using var bgraBitmap = SoftwareBitmap.Convert(
                        rawBitmap,
                        BitmapPixelFormat.Bgra8,
                        BitmapAlphaMode.Premultiplied);

                    if (_softwareBitmapSource != null)
                    {
                        await _softwareBitmapSource.SetBitmapAsync(bgraBitmap);
                    }
                }
                catch { }
                finally
                {
                    _isRenderingFrame = false;
                }
            });
        }

        private void OnTelemetryReceived(EngineTelemetry t)
        {
            DispatcherQueue.TryEnqueue(() =>
            {
                try
                {
                    HudFpsText.Text = $"{t.Fps:F1} FPS";
                    HudLatencyText.Text = $"{t.FrameTimeMs:F1} ms";

                    TelemetryGpuText.Text = string.IsNullOrEmpty(t.GpuName)
                        ? "Acelerador: DirectML GPU"
                        : $"Acelerador: {t.GpuName}";

                    TelemetryAiMsText.Text = $"IA Latencia: {t.AiMs:F1} ms";
                    TelemetryGpuMsText.Text = $"Procesamiento: {t.GpuMs:F1} ms";

                    if (t.AudioPeakDb > -90)
                    {
                        AudioVuMeter.Value = Math.Clamp(t.AudioPeakDb, -60, 0);
                        AudioDbText.Text = $"{t.AudioPeakDb:F0} dB";
                    }

                    if (t.Faces > 0)
                    {
                        HudFaceTrackText.Text = $"FaceMesh 3D ({t.Faces} Rostro)";
                    }
                    else
                    {
                        HudFaceTrackText.Text = "Esperando Rostro";
                    }

                    if (VirtualCamToggle.IsChecked != t.VcamActive)
                    {
                        VirtualCamToggle.IsChecked = t.VcamActive;
                        VirtualCamText.Text = t.VcamActive ? "Cámara Virtual Activa" : "Iniciar Cámara Virtual";
                    }
                }
                catch { }
            });
        }

        private void OnDevicesReceived(DevicesPayload dev)
        {
            _latestDevices = dev;
            DispatcherQueue.TryEnqueue(() =>
            {
                try
                {
                    var curCam = dev.Cameras.FirstOrDefault(c => c.Id == dev.SelectedCamera) ?? dev.Cameras.FirstOrDefault();
                    if (curCam != null)
                    {
                        SelectedCameraNameText.Text = curCam.Name;
                    }

                    if (dev.SelectedFormat != null)
                    {
                        SelectedFormatBadgeText.Text = $"{dev.SelectedFormat.Width}×{dev.SelectedFormat.Height} @ {dev.SelectedFormat.Fps} FPS • {dev.SelectedFormat.Format}";
                        HudResolutionText.Text = $"{dev.SelectedFormat.Width}×{dev.SelectedFormat.Height}";
                    }
                }
                catch { }
            });
        }

        private void OnPresetLoaded(StudioConfig cfg)
        {
            _currentConfig = cfg;
            DispatcherQueue.TryEnqueue(() =>
            {
                SyncUiFromConfig();
            });
        }

        // =====================================================================
        // 2. SIDEBAR NAVIGATION TABS
        // =====================================================================
        private void NavTab_Click(object sender, RoutedEventArgs e)
        {
            if (sender is not Button btn || btn.Tag is not string tabTag) return;

            // Reset all tab button styles
            ResetTabButton(TabMejorarButton);
            ResetTabButton(TabMorfologiaButton);
            ResetTabButton(TabLuzButton);
            ResetTabButton(TabFiltrosButton);
            ResetTabButton(TabFondoButton);

            // Highlight clicked button
            btn.Background = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 38, 26, 43));
            btn.BorderBrush = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 236, 72, 153));
            btn.BorderThickness = new Thickness(1);

            // Hide all panels
            PanelMejorar.Visibility = Visibility.Collapsed;
            PanelMorfologia.Visibility = Visibility.Collapsed;
            PanelLuz.Visibility = Visibility.Collapsed;
            PanelFiltros.Visibility = Visibility.Collapsed;
            PanelFondo.Visibility = Visibility.Collapsed;

            // Show selected panel
            switch (tabTag)
            {
                case "mejorar":
                    PanelMejorar.Visibility = Visibility.Visible;
                    break;
                case "morfologia":
                    PanelMorfologia.Visibility = Visibility.Visible;
                    break;
                case "luz":
                    PanelLuz.Visibility = Visibility.Visible;
                    break;
                case "filtros":
                    PanelFiltros.Visibility = Visibility.Visible;
                    break;
                case "fondo":
                    PanelFondo.Visibility = Visibility.Visible;
                    break;
            }
        }

        private void ResetTabButton(Button btn)
        {
            btn.Background = new SolidColorBrush(Windows.UI.Color.FromArgb(0, 0, 0, 0));
            btn.BorderThickness = new Thickness(0);
        }

        // =====================================================================
        // 3. BASE TONE CIRCULAR SWATCHES
        // =====================================================================
        private void BaseToneSwatch_Click(object sender, RoutedEventArgs e)
        {
            if (sender is not Button btn || btn.Tag is not string toneId) return;

            _currentConfig.FoundationTone = toneId;
            SendConfig();
        }

        // =====================================================================
        // 4. FILTER PRESETS (LUTS)
        // =====================================================================
        private void FilterPreset_Click(object sender, RoutedEventArgs e)
        {
            if (sender is not Button btn || btn.Tag is not string filterId) return;

            switch (filterId)
            {
                case "original":
                    _currentConfig.Saturation = 1.0f;
                    _currentConfig.Contrast = 1.0f;
                    _currentConfig.Exposure = 0.0f;
                    _currentConfig.WhiteBalance = 0.0f;
                    _currentConfig.Hue = 0.0f;
                    break;
                case "calidez_latina":
                    _currentConfig.WhiteBalance = 0.35f;
                    _currentConfig.Saturation = 1.15f;
                    _currentConfig.Contrast = 1.08f;
                    break;
                case "golden_glow":
                    _currentConfig.WhiteBalance = 0.50f;
                    _currentConfig.Exposure = 0.15f;
                    _currentConfig.Saturation = 1.20f;
                    _currentConfig.Contrast = 1.05f;
                    break;
                case "miami_glow":
                    _currentConfig.WhiteBalance = 0.20f;
                    _currentConfig.Saturation = 1.30f;
                    _currentConfig.Contrast = 1.12f;
                    _currentConfig.Hue = 0.02f;
                    break;
                case "bigo_pop":
                    _currentConfig.Brightness = 0.08f;
                    _currentConfig.Saturation = 1.25f;
                    _currentConfig.Contrast = 1.08f;
                    _currentConfig.Sharpness = 0.35f;
                    _currentConfig.SkinSmoothing = 0.65f;
                    _currentConfig.EyeBrighten = 0.45f;
                    _currentConfig.TeethWhitening = 0.50f;
                    break;
                case "cyberpunk":
                    _currentConfig.Contrast = 1.35f;
                    _currentConfig.Saturation = 1.40f;
                    _currentConfig.Hue = -0.15f;
                    break;
                case "film_moody":
                    _currentConfig.Contrast = 1.18f;
                    _currentConfig.Saturation = 0.85f;
                    _currentConfig.Exposure = -0.10f;
                    break;
                case "monochrome":
                    _currentConfig.Saturation = 0.0f;
                    _currentConfig.Contrast = 1.20f;
                    break;
            }

            SyncUiFromConfig();
            SendConfig();
        }

        // =====================================================================
        // 5. SLIDER EVENTS & LIVE PREVIEWS
        // =====================================================================
        private void BeautySlider_ValueChanged(object sender, Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;

            _currentConfig.SkinSmoothing = (float)SkinSmoothingSlider.Value;
            _currentConfig.TeethWhitening = (float)TeethSlider.Value;
            _currentConfig.EyeBrighten = (float)EyeBrightenSlider.Value;
            _currentConfig.FaceSlim = (float)FaceSlimSlider.Value;
            _currentConfig.EyeEnlarge = (float)EyeEnlargeSlider.Value;
            _currentConfig.BrowLift = (float)BrowLiftSlider.Value;
            _currentConfig.NoseSlim = (float)NoseSlimSlider.Value;
            _currentConfig.LipPlump = (float)LipPlumpSlider.Value;
            _currentConfig.Lipstick = (float)LipstickSlider.Value;
            _currentConfig.Contour = (float)ContourSlider.Value;
            _currentConfig.Eyeshadow = (float)EyeshadowSlider.Value;

            UpdateValueLabels();
            SendConfig();
        }

        private void FilterSlider_ValueChanged(object sender, Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;

            _currentConfig.Exposure = (float)ExposureSlider.Value;
            _currentConfig.Brightness = (float)BrightnessSlider.Value;
            _currentConfig.Contrast = (float)ContrastSlider.Value;
            _currentConfig.Saturation = (float)SaturationSlider.Value;
            _currentConfig.Sharpness = (float)SharpnessSlider.Value;
            _currentConfig.WhiteBalance = (float)WbSlider.Value;
            _currentConfig.Hue = (float)HueSlider.Value;

            UpdateValueLabels();
            SendConfig();
        }

        private void BgSlider_ValueChanged(object sender, Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;

            _currentConfig.BlurRadius = (float)BlurRadiusSlider.Value;
            BlurRadiusValText.Text = $"{_currentConfig.BlurRadius:F0} px";
            SendConfig();
        }

        private void UpdateValueLabels()
        {
            SkinSmoothingValText.Text = $"{_currentConfig.SkinSmoothing * 100:F0}%";
            TeethValText.Text = $"{_currentConfig.TeethWhitening * 100:F0}%";
            EyeBrightenValText.Text = $"{_currentConfig.EyeBrighten * 100:F0}%";
            FaceSlimValText.Text = $"{_currentConfig.FaceSlim * 100:F0}%";
            EyeEnlargeValText.Text = $"{_currentConfig.EyeEnlarge * 100:F0}%";
            BrowLiftValText.Text = $"{_currentConfig.BrowLift * 100:F0}%";
            NoseSlimValText.Text = $"{_currentConfig.NoseSlim:+0.00;-0.00;0.00}";
            LipPlumpValText.Text = $"{_currentConfig.LipPlump * 100:F0}%";
            LipstickValText.Text = $"{_currentConfig.Lipstick * 100:F0}%";
            ContourValText.Text = $"{_currentConfig.Contour * 100:F0}%";
            EyeshadowValText.Text = $"{_currentConfig.Eyeshadow * 100:F0}%";

            ExposureValText.Text = $"{_currentConfig.Exposure:+0.00;-0.00;0.00} EV";
            BrightnessValText.Text = $"{_currentConfig.Brightness:+0.00;-0.00;0.00}";
            ContrastValText.Text = $"{_currentConfig.Contrast:F2}";
            SaturationValText.Text = $"{_currentConfig.Saturation:F2}";
            SharpnessValText.Text = $"{_currentConfig.Sharpness:F2}";
            WbValText.Text = $"{_currentConfig.WhiteBalance:+0.00;-0.00;0.00}";
            HueValText.Text = $"{_currentConfig.Hue:+0.00;-0.00;0.00}";
        }

        // =====================================================================
        // 6. TOGGLES & SWITCHES
        // =====================================================================
        private void BeautyMasterSwitch_Toggled(object sender, RoutedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;
            _currentConfig.BeautyEnabled = BeautyMasterSwitch.IsOn;
            SendConfig();
        }

        private void AiAutoTuneSwitch_Toggled(object sender, RoutedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;
            _currentConfig.AiAutoTuneEnabled = AiAutoTuneSwitch.IsOn;
            SendConfig();
        }

        private void AiLightingSwitch_Toggled(object sender, RoutedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;
            _currentConfig.AiLightingEnabled = AiLightingSwitch.IsOn;
            SendConfig();
        }

        private void AiDenoiseSwitch_Toggled(object sender, RoutedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;
            _currentConfig.AiDenoiseEnabled = AiDenoiseSwitch.IsOn;
            SendConfig();
        }

        private void PersonSegmentationSwitch_Toggled(object sender, RoutedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;
            _currentConfig.PersonSegmentation = PersonSegmentationSwitch.IsOn;
            SendConfig();
        }

        private void BgModeComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (!_isInitialized || _isUpdatingUiFromBackend) return;
            if (BgModeComboBox.SelectedItem is ComboBoxItem item && int.TryParse(item.Tag?.ToString(), out int mode))
            {
                _currentConfig.BgMode = mode;
                SendConfig();
            }
        }

        // =====================================================================
        // 7. HEADER BUTTON ACTIONS
        // =====================================================================
        private void AutoTuneButton_Click(object sender, RoutedEventArgs e)
        {
            _currentConfig.AiAutoTuneEnabled = true;
            AiAutoTuneSwitch.IsOn = true;
            SendConfig();
        }

        private void PausePreviewToggle_Click(object sender, RoutedEventArgs e)
        {
            _currentConfig.PreviewPaused = PausePreviewToggle.IsChecked == true;
            SendConfig();
        }

        private void VirtualCamToggle_Click(object sender, RoutedEventArgs e)
        {
            bool requested = VirtualCamToggle.IsChecked == true;
            _currentConfig.VcamActive = requested;
            VirtualCamText.Text = requested ? "Cámara Virtual Activa" : "Iniciar Cámara Virtual";
            SendConfig();
        }

        private void MuteAudioButton_Click(object sender, RoutedEventArgs e)
        {
            _isAudioMuted = !_isAudioMuted;
            MuteAudioIcon.Glyph = _isAudioMuted ? "\uF781" : "\uE720";
            MuteAudioIcon.Foreground = _isAudioMuted
                ? new SolidColorBrush(Windows.UI.Color.FromArgb(255, 239, 68, 68))
                : new SolidColorBrush(Windows.UI.Color.FromArgb(255, 156, 163, 175));
        }

        // =====================================================================
        // 8. SOURCE PROPERTIES MODAL DIALOG
        // =====================================================================
        private async void SourcePropertiesButton_Click(object sender, RoutedEventArgs e)
        {
            if (_latestDevices == null) return;

            var dialog = new ContentDialog
            {
                Title = "Configuración de Fuente y Hardware",
                PrimaryButtonText = "Aplicar",
                CloseButtonText = "Cancelar",
                DefaultButton = ContentDialogButton.Primary,
                XamlRoot = this.XamlRoot
            };

            var stack = new StackPanel { Spacing = 14, Width = 380 };

            // Camera selector
            stack.Children.Add(new TextBlock { Text = "Cámara de Entrada", FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
            var camCombo = new ComboBox { HorizontalAlignment = HorizontalAlignment.Stretch };
            foreach (var cam in _latestDevices.Cameras)
            {
                camCombo.Items.Add(new ComboBoxItem { Content = cam.Name, Tag = cam.Id });
            }
            var selCamItem = camCombo.Items.Cast<ComboBoxItem>().FirstOrDefault(i => (string)i.Tag == _currentConfig.SelectedCameraId);
            camCombo.SelectedItem = selCamItem ?? camCombo.Items.FirstOrDefault();
            stack.Children.Add(camCombo);

            // Format selector
            stack.Children.Add(new TextBlock { Text = "Resolución y FPS", FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
            var fmtCombo = new ComboBox { HorizontalAlignment = HorizontalAlignment.Stretch };
            string? selectedCamId = ((ComboBoxItem?)camCombo.SelectedItem)?.Tag?.ToString();
            var activeCam = _latestDevices.Cameras.FirstOrDefault(c => c.Id == selectedCamId);
            if (activeCam != null)
            {
                foreach (var f in activeCam.Formats)
                {
                    fmtCombo.Items.Add(new ComboBoxItem
                    {
                        Content = $"{f.Width}×{f.Height} @ {f.Fps} FPS ({f.Format})",
                        Tag = f
                    });
                }
            }
            fmtCombo.SelectedIndex = 0;
            stack.Children.Add(fmtCombo);

            // Audio selector
            stack.Children.Add(new TextBlock { Text = "Micrófono de Estudio", FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
            var micCombo = new ComboBox { HorizontalAlignment = HorizontalAlignment.Stretch };
            foreach (var mic in _latestDevices.Microphones)
            {
                micCombo.Items.Add(new ComboBoxItem { Content = mic.Name, Tag = mic.Id });
            }
            micCombo.SelectedIndex = 0;
            stack.Children.Add(micCombo);

            dialog.Content = stack;

            var result = await dialog.ShowAsync();
            if (result == ContentDialogResult.Primary)
            {
                if (camCombo.SelectedItem is ComboBoxItem cItem && cItem.Tag is string camId)
                {
                    _currentConfig.SelectedCameraId = camId;
                    SelectedCameraNameText.Text = cItem.Content?.ToString() ?? "Cámara";
                }

                if (fmtCombo.SelectedItem is ComboBoxItem fItem && fItem.Tag is CameraFormatInfo fmt)
                {
                    _currentConfig.SelectedWidth = fmt.Width;
                    _currentConfig.SelectedHeight = fmt.Height;
                    _currentConfig.SelectedFps = fmt.Fps;
                    _currentConfig.SelectedFormat = fmt.Format;
                    SelectedFormatBadgeText.Text = $"{fmt.Width}×{fmt.Height} @ {fmt.Fps} FPS • {fmt.Format}";
                }

                SendConfig();
            }
        }

        // =====================================================================
        // 9. CONFIG SYNC & TRANSMISSION
        // =====================================================================
        private void SyncUiFromConfig()
        {
            _isUpdatingUiFromBackend = true;
            try
            {
                BeautyMasterSwitch.IsOn = _currentConfig.BeautyEnabled;
                SkinSmoothingSlider.Value = _currentConfig.SkinSmoothing;
                TeethSlider.Value = _currentConfig.TeethWhitening;
                EyeBrightenSlider.Value = _currentConfig.EyeBrighten;
                FaceSlimSlider.Value = _currentConfig.FaceSlim;
                EyeEnlargeSlider.Value = _currentConfig.EyeEnlarge;
                BrowLiftSlider.Value = _currentConfig.BrowLift;
                NoseSlimSlider.Value = _currentConfig.NoseSlim;
                LipPlumpSlider.Value = _currentConfig.LipPlump;
                LipstickSlider.Value = _currentConfig.Lipstick;
                ContourSlider.Value = _currentConfig.Contour;
                EyeshadowSlider.Value = _currentConfig.Eyeshadow;

                ExposureSlider.Value = _currentConfig.Exposure;
                BrightnessSlider.Value = _currentConfig.Brightness;
                ContrastSlider.Value = _currentConfig.Contrast;
                SaturationSlider.Value = _currentConfig.Saturation;
                SharpnessSlider.Value = _currentConfig.Sharpness;
                WbSlider.Value = _currentConfig.WhiteBalance;
                HueSlider.Value = _currentConfig.Hue;

                AiAutoTuneSwitch.IsOn = _currentConfig.AiAutoTuneEnabled;
                AiLightingSwitch.IsOn = _currentConfig.AiLightingEnabled;
                AiDenoiseSwitch.IsOn = _currentConfig.AiDenoiseEnabled;
                PersonSegmentationSwitch.IsOn = _currentConfig.PersonSegmentation;

                BlurRadiusSlider.Value = _currentConfig.BlurRadius;

                UpdateValueLabels();
            }
            finally
            {
                _isUpdatingUiFromBackend = false;
            }
        }

        private void SendConfig()
        {
            _engineService.SendConfig(_currentConfig);
        }
    }
}

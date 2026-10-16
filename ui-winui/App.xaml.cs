using Windows.ApplicationModel;
using Windows.ApplicationModel.Activation;
using Windows.Foundation;
using Windows.Foundation.Collections;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;
using Microsoft.UI.Xaml.Shapes;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace CamArStudio_WinUI;

/// <summary>
/// Provides application-specific behavior to supplement the default Application class.
/// </summary>
public partial class App : Application
{
    private Window? _window;
    public static MainWindow? MainWindowInstance { get; private set; }

    public App()
    {
        try { System.IO.File.WriteAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] App constructor\n"); } catch { }

        this.UnhandledException += (sender, e) =>
        {
            try
            {
                System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] UnhandledException: {e.Message}\n{e.Exception}\n");
            }
            catch { }
        };
        InitializeComponent();
    }

    protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
    {
        try
        {
            System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] OnLaunched start\n");
            var mainWindow = new MainWindow();
            _window = mainWindow;
            MainWindowInstance = mainWindow;

            // Set size and center cleanly
            try
            {
                var appWindow = mainWindow.AppWindow;
                int width = 1420;
                int height = 880;
                appWindow.Resize(new Windows.Graphics.SizeInt32(width, height));
                var displayArea = Microsoft.UI.Windowing.DisplayArea.GetFromWindowId(appWindow.Id, Microsoft.UI.Windowing.DisplayAreaFallback.Primary);
                if (displayArea != null)
                {
                    var pos = appWindow.Position;
                    pos.X = Math.Max(20, (displayArea.WorkArea.Width - width) / 2);
                    pos.Y = Math.Max(20, (displayArea.WorkArea.Height - height) / 2);
                    appWindow.Move(pos);
                }
            }
            catch { }

            _window.Activate();
            System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] OnLaunched _window.Activate() completed\n");
        }
        catch (Exception ex)
        {
            try
            {
                System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] OnLaunched Exception: {ex.Message}\n{ex.StackTrace}\n");
            }
            catch { }
            throw;
        }
    }
}

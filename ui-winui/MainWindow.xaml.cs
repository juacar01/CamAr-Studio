using System;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;

namespace CamArStudio_WinUI;

public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        try { System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] MainWindow constructor begin\n"); } catch { }
        try
        {
            InitializeComponent();
            Title = "CamAr Studio - Pro Edition";
            AppWindow.Title = "CamAr Studio - Pro Edition";

            this.Content = new MainPage();

            this.Closed += (s, e) =>
            {
                try { System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] MainWindow.Closed event fired!\n"); } catch { }
            };

            try { System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] MainWindow constructor complete\n"); } catch { }
        }
        catch (Exception ex)
        {
            try { System.IO.File.AppendAllText(@"D:\proyectos\CamAr Studio\app_lifecycle.log", $"[{DateTime.Now:HH:mm:ss.fff}] MainWindow Exception: {ex.Message}\n{ex.StackTrace}\n"); } catch { }
            throw;
        }
    }
}

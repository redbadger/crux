using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;
using SharedTypes;
using Windows.Foundation;
using Windows.Foundation.Collections;
using static System.Formats.Asn1.AsnWriter;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace SimpleCounter
{
    /// <summary>
    /// An empty window that can be used on its own or navigated to within a Frame.
    /// </summary>
    public sealed partial class MainWindow : Window
    {
        private readonly Core _core;

        public MainWindow()
        {
            this.InitializeComponent();
            _core = new Core();
            UpdateUI();
        }

        private void UpdateUI()
        {
            // Update the UI based on the ViewModel
            CounterText.Text = _core.View.count;
        }

        private void ResetButton_Click(object sender, RoutedEventArgs e)
        {
            _core.Update(new Event.Reset());
            UpdateUI();
        }

        private void IncrementButton_Click(object sender, RoutedEventArgs e)
        {
            _core.Update(new Event.Increment());
            UpdateUI();
        }

        private void DecrementButton_Click(object sender, RoutedEventArgs e)
        {
            _core.Update(new Event.Decrement());
            UpdateUI();
        }
    }
}

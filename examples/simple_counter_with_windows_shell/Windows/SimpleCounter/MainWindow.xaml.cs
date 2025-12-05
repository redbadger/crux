using Microsoft.UI.Xaml;
using SharedTypes;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace SimpleCounter {
    /// <summary>
    /// An empty window that can be used on its own or navigated to within a Frame.
    /// </summary>
    internal sealed partial class MainWindow : Window {
        public Core Core { get; }

        public MainWindow() {
            this.InitializeComponent();
            Core = new Core();
        }

        private void ResetButton_Click(object sender, RoutedEventArgs e) {
            Core.Update(new Event.Reset());
        }

        private void IncrementButton_Click(object sender, RoutedEventArgs e) {
            Core.Update(new Event.Increment());
        }

        private void DecrementButton_Click(object sender, RoutedEventArgs e) {
            Core.Update(new Event.Decrement());
        }
    }
}

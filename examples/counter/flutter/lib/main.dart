import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'package:counter_flutter/src/rust/frb_generated.dart';

import 'src/core/core.dart';
import 'src/core/types.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();

  runApp(
    ChangeNotifierProvider(create: (_) => Core(), child: const CounterApp()),
  );
}

class CounterApp extends StatelessWidget {
  const CounterApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Crux Counter',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.teal),
        useMaterial3: true,
      ),
      home: const CounterScreen(),
    );
  }
}

class CounterScreen extends StatelessWidget {
  const CounterScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final label = context.watch<Core>().viewModel.count;

    return Scaffold(
      appBar: AppBar(title: const Text('Crux Counter')),
      body: Center(
        child: Text(
          label,
          style: Theme.of(context).textTheme.headlineMedium,
          textAlign: TextAlign.center,
        ),
      ),
      persistentFooterButtons: [
        IconButton(
          icon: const Icon(Icons.remove),
          tooltip: 'Decrement',
          onPressed: () => context.read<Core>().update(EventDecrement()),
        ),
        IconButton(
          icon: const Icon(Icons.add),
          tooltip: 'Increment',
          onPressed: () => context.read<Core>().update(EventIncrement()),
        ),
        TextButton(
          onPressed: () => context.read<Core>().update(EventReset()),
          child: const Text('Reset'),
        ),
      ],
    );
  }
}

// Full app tests need a native `counter_flutter` library (FRB); `flutter test` on the
// VM does not load it. Run the app with `just run` on a device / desktop target.

import 'package:flutter_test/flutter_test.dart';

void main() {
  test('sanity', () {
    expect(2 + 2, 4);
  });
}

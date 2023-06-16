import { spawnSync } from "child_process";
import { inspect } from "util";
import { Disposable, commands, window } from "vscode";

export interface IDisposable {
  dispose(): void;
}

export class DisposableStore {
  private disposables = new Set<Disposable>();

  add(disposable: Disposable): void {
    this.disposables.add(disposable);
  }

  dispose(): void {
    for (const disposable of this.disposables) {
      disposable.dispose();
    }

    this.disposables.clear();
  }
}

export const log = new (class {
  private enabled = true;
  private readonly output = window.createOutputChannel("Crux Analyzer Client");

  setEnabled(yes: boolean): void {
    log.enabled = yes;
  }

  // Hint: the type [T, ...T[]] means a non-empty array
  debug(...msg: [unknown, ...unknown[]]): void {
    if (!log.enabled) return;
    log.write("DEBUG", ...msg);
  }

  info(...msg: [unknown, ...unknown[]]): void {
    log.write("INFO", ...msg);
  }

  warn(...msg: [unknown, ...unknown[]]): void {
    // debugger;
    log.write("WARN", ...msg);
  }

  error(...msg: [unknown, ...unknown[]]): void {
    debugger;
    log.write("ERROR", ...msg);
    log.output.show(true);
  }

  private write(label: string, ...messageParts: unknown[]): void {
    const message = messageParts.map(log.stringify).join(" ");
    const dateTime = new Date().toLocaleString();
    log.output.appendLine(`${label} [${dateTime}]: ${message}`);
  }

  private stringify(val: unknown): string {
    if (typeof val === "string") return val;
    return inspect(val, {
      colors: false,
      depth: 6, // heuristic
    });
  }
})();

export function isValidExecutable(path: string): boolean {
  log.debug("Checking availability of a binary at", path);

  const res = spawnSync(path, ["--version"], { encoding: "utf8" });

  const printOutput = res.error ? log.warn : log.info;
  printOutput(path, "--version:", res);

  return res.status === 0;
}

export function setContextValue(key: string, value: any): Thenable<void> {
  return commands.executeCommand("setContext", key, value);
}

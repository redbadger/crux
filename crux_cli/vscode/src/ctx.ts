import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";
import { createClient } from "./client";
import { isValidExecutable, log } from "./util";
import { ServerStatusParams, serverStatus } from "./lsp_ext";

const SERVER_CONFIG = "crux-analyzer.server";

export type CommandFactory = {
  enabled: (ctx: CtxInit) => Cmd;
  disabled?: (ctx: Ctx) => Cmd;
};

export type CtxInit = Ctx & {
  readonly client: lc.LanguageClient;
};

export class Ctx {
  readonly statusBar: vscode.StatusBarItem;

  private _client: lc.LanguageClient | undefined;
  private _serverPath: string | undefined;
  private outputChannel: vscode.OutputChannel | undefined;
  private clientSubscriptions: Disposable[];
  private commandFactories: Record<string, CommandFactory>;
  private commandDisposables: Disposable[];

  get client() {
    return this._client;
  }

  constructor(
    readonly extCtx: vscode.ExtensionContext,
    commandFactories: Record<string, CommandFactory>
  ) {
    extCtx.subscriptions.push(this);
    this.statusBar = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Left
    );
    this.clientSubscriptions = [];
    this.commandDisposables = [];
    this.commandFactories = commandFactories;

    this.updateCommands("disable");
    this.setServerStatus({
      health: "stopped",
    });
  }

  dispose() {
    this.statusBar.dispose();
    void this.disposeClient();
    this.commandDisposables.forEach((disposable) => disposable.dispose());
  }

  private async getOrCreateClient() {
    if (!this.outputChannel) {
      this.outputChannel = vscode.window.createOutputChannel(
        "Crux Analyzer Language Server"
      );
      this.pushExtCleanup(this.outputChannel);
    }

    if (!this._client) {
      this._serverPath = vscode.workspace
        .getConfiguration(SERVER_CONFIG)
        .get("path")!;
      const run: lc.Executable = {
        command: this._serverPath,
        args: ["lsp"],
      };
      const serverOptions = {
        run,
        debug: run,
      };

      this._client = await createClient(this.outputChannel, serverOptions);
      this.pushClientCleanup(
        this._client.onNotification(serverStatus, (params: any) =>
          this.setServerStatus(params)
        )
      );
    }
    return this._client;
  }

  async start() {
    log.info("Starting language client");
    const client = await this.getOrCreateClient();
    if (!client) {
      return;
    }
    await client.start();
    this.updateCommands();
  }

  async restart() {
    // FIXME: We should re-use the client, that is ctx.deactivate() if none of the configs have changed
    await this.stopAndDispose();
    await this.start();
  }

  async stop() {
    if (!this._client) {
      return;
    }
    log.info("Stopping language client");
    this.updateCommands("disable");
    await this._client.stop();
  }

  async stopAndDispose() {
    if (!this._client) {
      return;
    }
    log.info("Disposing language client");
    this.updateCommands("disable");
    await this.disposeClient();
    this.setServerStatus({
      health: "stopped",
    });
  }

  private async disposeClient() {
    this.clientSubscriptions?.forEach((disposable) => disposable.dispose());
    this.clientSubscriptions = [];
    await this._client?.dispose();
    this._serverPath = undefined;
    this._client = undefined;
  }

  get extensionPath(): string {
    return this.extCtx.extensionPath;
  }

  get subscriptions(): Disposable[] {
    return this.extCtx.subscriptions;
  }

  get serverPath(): string | undefined {
    return this._serverPath;
  }

  private updateCommands(forceDisable?: "disable") {
    this.commandDisposables.forEach((disposable) => disposable.dispose());
    this.commandDisposables = [];

    const clientRunning = (!forceDisable && this._client?.isRunning()) ?? false;
    const isClientRunning = function (_ctx: Ctx): _ctx is CtxInit {
      return clientRunning;
    };

    for (const [name, factory] of Object.entries(this.commandFactories)) {
      const fullName = `crux-analyzer.${name}`;
      let callback;
      if (isClientRunning(this)) {
        // we asserted that `client` is defined
        callback = factory.enabled(this);
      } else if (factory.disabled) {
        callback = factory.disabled(this);
      } else {
        callback = () =>
          vscode.window.showErrorMessage(
            `command ${fullName} failed: crux-analyzer server is not running`
          );
      }

      this.commandDisposables.push(
        vscode.commands.registerCommand(fullName, callback)
      );
    }
  }

  setServerStatus(status: ServerStatusParams | { health: "stopped" }) {
    let icon = "";
    const statusBar = this.statusBar;
    statusBar.show();
    statusBar.tooltip = new vscode.MarkdownString("", true);
    statusBar.tooltip.isTrusted = true;
    switch (status.health) {
      case "ok":
        statusBar.tooltip.appendText(status.message ?? "Ready");
        statusBar.color = undefined;
        statusBar.backgroundColor = undefined;
        statusBar.command = "crux-analyzer.stopServer";
        break;
      case "warning":
        if (status.message) {
          statusBar.tooltip.appendText(status.message);
        }
        statusBar.color = new vscode.ThemeColor(
          "statusBarItem.warningForeground"
        );
        statusBar.backgroundColor = new vscode.ThemeColor(
          "statusBarItem.warningBackground"
        );
        statusBar.command = "crux-analyzer.openLogs";
        icon = "$(warning) ";
        break;
      case "error":
        if (status.message) {
          statusBar.tooltip.appendText(status.message);
        }
        statusBar.color = new vscode.ThemeColor(
          "statusBarItem.errorForeground"
        );
        statusBar.backgroundColor = new vscode.ThemeColor(
          "statusBarItem.errorBackground"
        );
        statusBar.command = "crux-analyzer.openLogs";
        icon = "$(error) ";
        break;
      case "stopped":
        statusBar.tooltip.appendText("Server is stopped");
        statusBar.tooltip.appendMarkdown(
          "\n\n[Start server](command:crux-analyzer.startServer)"
        );
        statusBar.color = undefined;
        statusBar.backgroundColor = undefined;
        statusBar.command = "crux-analyzer.startServer";
        statusBar.text = `$(stop-circle) crux-analyzer`;
        return;
    }
    if (statusBar.tooltip.value) {
      statusBar.tooltip.appendText("\n\n");
    }
    statusBar.tooltip.appendMarkdown(
      "\n\n[Open logs](command:crux-analyzer.openLogs)"
    );
    statusBar.tooltip.appendMarkdown(
      "\n\n[Restart server](command:crux-analyzer.restartServer)"
    );
    statusBar.tooltip.appendMarkdown(
      "\n\n[Stop server](command:crux-analyzer.stopServer)"
    );
    if (!status.quiescent) icon = "$(sync~spin) ";
    statusBar.text = `${icon}crux-analyzer`;
  }

  pushExtCleanup(d: Disposable) {
    this.extCtx.subscriptions.push(d);
  }

  private pushClientCleanup(d: Disposable) {
    this.clientSubscriptions.push(d);
  }
}

export interface Disposable {
  dispose(): void;
}

export type Cmd = (...args: any[]) => unknown;

export async function onDidChangeConfiguration(
  e: vscode.ConfigurationChangeEvent
): Promise<void> {
  if (e.affectsConfiguration(SERVER_CONFIG)) {
    const command: string = vscode.workspace
      .getConfiguration(SERVER_CONFIG)
      .get("path")!;

    if (isValidExecutable(command)) {
      log.info("Restarting server due to configuration change");
      await vscode.commands.executeCommand("crux-analyzer.restartServer");
    } else {
      void vscode.window.showErrorMessage(
        `Crux Analyzer: Invalid executable path: ${command}`
      );
    }
  }
}

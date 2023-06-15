import { ExtensionContext, commands, window, workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient;

const SERVER_CONFIG = "crux-analyzer.server";

export async function activate(context: ExtensionContext) {
  const serverConfig = workspace.getConfiguration(SERVER_CONFIG);
  const command: string = serverConfig.get("path")!;
  const args: [string] = serverConfig.get("args")!;

  const serverOptions: ServerOptions = { command, args };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "toml" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/Crux.toml"),
    },
    diagnosticCollectionName: "crux-analyzer",
  };

  client = new LanguageClient(
    "crux-analyzer",
    "Crux Analyzer",
    serverOptions,
    clientOptions
  );

  await start();

  context.subscriptions.push(
    commands.registerCommand("crux.restartServer", restart)
  );

  // this still needs some work...
  // workspace.onDidChangeConfiguration(
  //   async (e) => {
  //     if (e.affectsConfiguration(SERVER_CONFIG)) {
  //       await restart();
  //     }
  //   },
  //   null,
  //   context.subscriptions
  // );
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }

  return client.stop();
}

async function start(): Promise<void> {
  await client.start();
  void window.showInformationMessage("Crux Analyzer started");
}

async function restart(): Promise<void> {
  await client.restart();
  void window.showInformationMessage("Crux Analyzer restarted");
}

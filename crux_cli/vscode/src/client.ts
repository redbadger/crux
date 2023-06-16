import { OutputChannel, workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

export async function createClient(
  outputChannel: OutputChannel,
  serverOptions: ServerOptions
): Promise<LanguageClient> {
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "toml" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/Crux.toml"),
    },
    diagnosticCollectionName: "crux-analyzer",
    outputChannel,
    middleware: {
      workspace: {
        // HACK: This is a workaround, when the client has been disposed, VSCode
        // continues to emit events to the client and the default one for this event
        // attempt to restart the client for no reason
        async didChangeWatchedFile(event, next) {
          if (client.isRunning()) {
            await next(event);
          }
        },
      },
    },
    markdown: {
      supportHtml: true,
    },
  };

  const client = new LanguageClient(
    "crux-analyzer",
    "Crux Analyzer Language Server",
    serverOptions,
    clientOptions
  );

  return client;
}

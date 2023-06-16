import {
  ConfigurationTarget,
  ExtensionContext,
  window,
  workspace,
} from "vscode";
import {
  DidChangeConfigurationNotification,
  LanguageClient,
} from "vscode-languageclient/node";
import { openLogs } from "./commands";
import { CommandFactory, Ctx, onDidChangeConfiguration } from "./ctx";
import { setContextValue } from "./util";

const CRUX_PROJECT_CONTEXT_NAME = "inCruxProject";

export interface CruxAnalyzerExtensionApi {
  readonly client?: LanguageClient;
}

export async function deactivate() {
  await setContextValue(CRUX_PROJECT_CONTEXT_NAME, undefined);
}

export async function activate(context: ExtensionContext) {
  const ctx = new Ctx(context, createCommands());
  // VS Code doesn't show a notification when an extension fails to activate
  // so we do it ourselves.
  const api = await activateServer(ctx).catch((err) => {
    void window.showErrorMessage(
      `Cannot activate crux-analyzer extension: ${err.message}`
    );
    throw err;
  });
  await setContextValue(CRUX_PROJECT_CONTEXT_NAME, true);
  return api;
}

async function activateServer(ctx: Ctx): Promise<CruxAnalyzerExtensionApi> {
  workspace.onDidChangeConfiguration(
    async (e) => {
      await ctx.client?.sendNotification(
        DidChangeConfigurationNotification.type,
        {
          settings: "",
        }
      );
      await onDidChangeConfiguration(e);
    },
    null,
    ctx.subscriptions
  );

  await ctx.start();
  return ctx;
}

function createCommands(): Record<string, CommandFactory> {
  return {
    restartServer: {
      enabled: (ctx) => async () => {
        await ctx.restart();
      },
      disabled: (ctx) => async () => {
        await ctx.start();
      },
    },
    startServer: {
      enabled: (ctx) => async () => {
        await ctx.start();
      },
      disabled: (ctx) => async () => {
        await ctx.start();
      },
    },
    stopServer: {
      enabled: (ctx) => async () => {
        // FIXME: We should re-use the client, that is ctx.deactivate() if none of the configs have changed
        await ctx.stopAndDispose();
        // ctx.setServerStatus({
        //   health: "stopped",
        // });
      },
      disabled: (_) => async () => {},
    },
    restoreDefaults: {
      enabled: (_) => async () => {
        await workspace
          .getConfiguration("crux-analyzer")
          .update("server.path", "crux", ConfigurationTarget.Global);
      },
      disabled: (_) => async () => {
        await workspace
          .getConfiguration("crux-analyzer")
          .update("server.path", "crux", ConfigurationTarget.Global);
      },
    },
    openLogs: { enabled: openLogs },
  };
}

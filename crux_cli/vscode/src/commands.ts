import { Cmd, CtxInit } from "./ctx";

export function openLogs(ctx: CtxInit): Cmd {
  return async () => {
    if (ctx.client.outputChannel) {
      ctx.client.outputChannel.show();
    }
  };
}

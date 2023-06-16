import { NotificationType } from "vscode-languageclient";

export const serverStatus = new NotificationType<ServerStatusParams>(
  "experimental/serverStatus"
);

export type ServerStatusParams = {
  health: "ok" | "warning" | "error";
  quiescent: boolean;
  message?: string;
};

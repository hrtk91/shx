import {
  ExtensionContext,
  workspace,
} from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext) {
  const command = workspace.getConfiguration("shx").get<string>("serverPath", "shx");

  const serverOptions: ServerOptions = {
    command,
    args: ["lsp"],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "shx" },
      { scheme: "file", language: "bashx" },
    ],
  };

  client = new LanguageClient("shx", "shx LSP", serverOptions, clientOptions);
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}

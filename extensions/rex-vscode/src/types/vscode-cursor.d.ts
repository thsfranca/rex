declare module "vscode" {
  export namespace cursor {
    export namespace plugins {
      export function registerPath(path: string): Promise<void> | void;
    }

    export namespace mcp {
      export interface McpServerConfig {
        name: string;
        command: string;
        args?: readonly string[];
        env?: Readonly<Record<string, string>>;
      }
      export function registerServer(config: McpServerConfig): Promise<void> | void;
    }
  }
}

export {};

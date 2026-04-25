import type { ExtensionToWebview, WebviewToExtension } from "../src/shared/messages";

interface VsCodeApi {
  postMessage(message: WebviewToExtension): void;
  getState<T>(): T | undefined;
  setState<T>(state: T): void;
}

declare global {
  interface Window {
    acquireVsCodeApi?: () => VsCodeApi;
  }
}

let cachedApi: VsCodeApi | undefined;

function getVsCodeApi(): VsCodeApi {
  if (cachedApi !== undefined) {
    return cachedApi;
  }
  if (typeof window.acquireVsCodeApi !== "function") {
    throw new Error("acquireVsCodeApi is unavailable; are we running outside a VS Code webview?");
  }
  cachedApi = window.acquireVsCodeApi();
  return cachedApi;
}

export function postToHost(message: WebviewToExtension): void {
  getVsCodeApi().postMessage(message);
}

export type InboundListener = (message: ExtensionToWebview) => void;

export function subscribeToHost(listener: InboundListener): () => void {
  const handler = (event: MessageEvent<ExtensionToWebview>): void => {
    const data = event.data;
    if (!isInboundMessage(data)) {
      return;
    }
    listener(data);
  };
  window.addEventListener("message", handler);
  return () => {
    window.removeEventListener("message", handler);
  };
}

function isInboundMessage(value: unknown): value is ExtensionToWebview {
  if (value === null || typeof value !== "object") {
    return false;
  }
  return typeof (value as { type?: unknown }).type === "string";
}

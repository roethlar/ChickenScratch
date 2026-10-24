/**
 * Endpoints that exist only on the `chikn-web` server. The browser has no
 * native folder picker, so the Welcome screen asks the server which projects
 * live under its root and where that root is.
 */
import { webGet } from "./transport";

export interface WebProject {
  name: string;
  path: string;
}

export interface WebInfo {
  root: string;
  version: string;
  events: boolean;
}

export function listWebProjects(): Promise<WebProject[]> {
  return webGet<WebProject[]>("/api/projects");
}

export function getWebInfo(): Promise<WebInfo> {
  return webGet<WebInfo>("/api/info");
}

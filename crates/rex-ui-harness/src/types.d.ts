declare module "delta-e" {
  export function getDeltaE00(
    lab1: { L: number; A: number; B: number },
    lab2: { L: number; A: number; B: number }
  ): number;
}

declare module "looks-same" {
  interface LooksSameOptions {
    tolerance?: number;
    antialiasingTolerance?: number;
    shouldCluster?: boolean;
  }
  interface LooksSameResult {
    equal: boolean;
    diffBounds?: unknown;
    diffClusters?: unknown;
  }
  function looksSame(
    img1: string | Buffer,
    img2: string | Buffer,
    options?: LooksSameOptions
  ): Promise<LooksSameResult>;
  export default looksSame;
}

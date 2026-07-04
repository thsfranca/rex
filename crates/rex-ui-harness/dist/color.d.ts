export interface Rgb {
    r: number;
    g: number;
    b: number;
}
export declare function parseCssColor(css: string): Rgb;
export declare function rgbToLab(rgb: Rgb): {
    L: number;
    A: number;
    B: number;
};
export declare function ciede2000(a: Rgb, b: Rgb): number;

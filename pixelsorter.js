// Pixel Sorter Engine - Ported from Rust
class PixelSorter {
    constructor() {
        this.algorithms = ['Horizontal', 'Vertical', 'Diagonal'];
        this.modes = ['Brightness', 'Black', 'White'];
    }

    /**
     * Main sorting function
     * @param {ImageData} imageData - Canvas ImageData
     * @param {string} algorithm - 'Horizontal', 'Vertical', or 'Diagonal'
     * @param {Object} params - { threshold, hueShift, sortMode }
     * @returns {ImageData} - Sorted ImageData
     */
    sortPixels(imageData, algorithm, params) {
        const { threshold = 0, hueShift = 0, sortMode = 'Brightness' } = params;
        const result = new ImageData(
            new Uint8ClampedArray(imageData.data),
            imageData.width,
            imageData.height
        );

        // Apply hue shift first if needed
        if (hueShift !== 0) {
            this.applyHueShift(result, hueShift);
        }

        // Apply sorting based on algorithm
        switch (algorithm) {
            case 'Horizontal':
                this.sortHorizontal(result, threshold, sortMode);
                break;
            case 'Vertical':
                this.sortVertical(result, threshold, sortMode);
                break;
            case 'Diagonal':
                this.sortDiagonal(result, threshold, sortMode);
                break;
        }

        return result;
    }

    /**
     * Sort pixels horizontally row by row
     */
    sortHorizontal(imageData, threshold, sortMode) {
        const { width, height, data } = imageData;
        
        for (let y = 0; y < height; y++) {
            const rowPixels = [];
            for (let x = 0; x < width; x++) {
                const idx = (y * width + x) * 4;
                rowPixels.push({
                    r: data[idx],
                    g: data[idx + 1],
                    b: data[idx + 2],
                    a: data[idx + 3]
                });
            }

            const intervals = this.findIntervals(rowPixels, threshold);
            
            for (const [start, end] of intervals) {
                if (end - start > 1) {
                    const segment = rowPixels.slice(start, end);
                    segment.sort((a, b) => this.sortKey(a, sortMode) - this.sortKey(b, sortMode));
                    
                    for (let i = 0; i < segment.length; i++) {
                        const x = start + i;
                        const idx = (y * width + x) * 4;
                        data[idx] = segment[i].r;
                        data[idx + 1] = segment[i].g;
                        data[idx + 2] = segment[i].b;
                        data[idx + 3] = segment[i].a;
                    }
                }
            }
        }
    }

    /**
     * Sort pixels vertically column by column
     */
    sortVertical(imageData, threshold, sortMode) {
        const { width, height, data } = imageData;
        
        for (let x = 0; x < width; x++) {
            const colPixels = [];
            for (let y = 0; y < height; y++) {
                const idx = (y * width + x) * 4;
                colPixels.push({
                    r: data[idx],
                    g: data[idx + 1],
                    b: data[idx + 2],
                    a: data[idx + 3]
                });
            }

            const intervals = this.findIntervals(colPixels, threshold);
            
            for (const [start, end] of intervals) {
                if (end - start > 1) {
                    const segment = colPixels.slice(start, end);
                    segment.sort((a, b) => this.sortKey(a, sortMode) - this.sortKey(b, sortMode));
                    
                    for (let i = 0; i < segment.length; i++) {
                        const y = start + i;
                        const idx = (y * width + x) * 4;
                        data[idx] = segment[i].r;
                        data[idx + 1] = segment[i].g;
                        data[idx + 2] = segment[i].b;
                        data[idx + 3] = segment[i].a;
                    }
                }
            }
        }
    }

    /**
     * Sort pixels diagonally
     */
    sortDiagonal(imageData, threshold, sortMode) {
        const { width, height, data } = imageData;
        
        // Process all diagonals (top-left to bottom-right)
        for (let offset = -height; offset < width; offset++) {
            const diagonalPixels = [];
            const positions = [];
            
            if (offset >= 0) {
                for (let i = 0; i < Math.min(height, width - offset); i++) {
                    const x = i + offset;
                    const y = i;
                    const idx = (y * width + x) * 4;
                    diagonalPixels.push({
                        r: data[idx],
                        g: data[idx + 1],
                        b: data[idx + 2],
                        a: data[idx + 3]
                    });
                    positions.push({ x, y });
                }
            } else {
                for (let i = 0; i < Math.min(width, height + offset); i++) {
                    const x = i;
                    const y = i - offset;
                    const idx = (y * width + x) * 4;
                    diagonalPixels.push({
                        r: data[idx],
                        g: data[idx + 1],
                        b: data[idx + 2],
                        a: data[idx + 3]
                    });
                    positions.push({ x, y });
                }
            }

            if (diagonalPixels.length <= 1) continue;

            const intervals = this.findIntervals(diagonalPixels, threshold);
            
            for (const [start, end] of intervals) {
                if (end - start > 1) {
                    const segment = diagonalPixels.slice(start, end);
                    segment.sort((a, b) => this.sortKey(a, sortMode) - this.sortKey(b, sortMode));
                    
                    for (let i = 0; i < segment.length; i++) {
                        const { x, y } = positions[start + i];
                        const idx = (y * width + x) * 4;
                        data[idx] = segment[i].r;
                        data[idx + 1] = segment[i].g;
                        data[idx + 2] = segment[i].b;
                        data[idx + 3] = segment[i].a;
                    }
                }
            }
        }
    }

    /**
     * Find intervals in pixel array based on brightness threshold
     */
    findIntervals(pixels, threshold) {
        if (pixels.length <= 1) return [];

        const intervals = [];
        let start = 0;

        for (let i = 1; i < pixels.length; i++) {
            const brightnessDiff = Math.abs(
                this.pixelBrightness(pixels[i]) - this.pixelBrightness(pixels[i - 1])
            );

            if (brightnessDiff > threshold) {
                if (i - start > 1) {
                    intervals.push([start, i]);
                }
                start = i;
            }
        }

        // Add final interval
        if (pixels.length - start > 1) {
            intervals.push([start, pixels.length]);
        }

        return intervals;
    }

    /**
     * Calculate pixel brightness using standard RGB to grayscale conversion
     */
    pixelBrightness(pixel) {
        return 0.299 * pixel.r + 0.587 * pixel.g + 0.114 * pixel.b;
    }

    /**
     * Get sort key based on sort mode
     */
    sortKey(pixel, mode) {
        switch (mode) {
            case 'Brightness':
                return this.pixelBrightness(pixel);
            case 'Black':
                return pixel.r;
            case 'White':
                return 255 - pixel.r;
            default:
                return this.pixelBrightness(pixel);
        }
    }

    /**
     * Apply hue shift to entire image
     */
    applyHueShift(imageData, hueShift) {
        const { width, height, data } = imageData;
        
        for (let i = 0; i < width * height; i++) {
            const idx = i * 4;
            const shifted = this.shiftPixelHue(
                { r: data[idx], g: data[idx + 1], b: data[idx + 2] },
                hueShift
            );
            data[idx] = shifted.r;
            data[idx + 1] = shifted.g;
            data[idx + 2] = shifted.b;
        }
    }

    /**
     * Shift hue of a single pixel
     */
    shiftPixelHue(pixel, hueShift) {
        // Convert RGB to HSV
        const r = pixel.r / 255;
        const g = pixel.g / 255;
        const b = pixel.b / 255;

        const max = Math.max(r, g, b);
        const min = Math.min(r, g, b);
        const delta = max - min;

        let h = 0;
        if (delta !== 0) {
            if (max === r) {
                h = 60 * (((g - b) / delta) % 6);
            } else if (max === g) {
                h = 60 * (((b - r) / delta) + 2);
            } else {
                h = 60 * (((r - g) / delta) + 4);
            }
        }

        if (h < 0) h += 360;

        const s = max === 0 ? 0 : delta / max;
        const v = max;

        // Apply hue shift
        h = (h + hueShift) % 360;
        if (h < 0) h += 360;

        // Convert HSV back to RGB
        const c = v * s;
        const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
        const m = v - c;

        let rPrime, gPrime, bPrime;
        if (h < 60) {
            [rPrime, gPrime, bPrime] = [c, x, 0];
        } else if (h < 120) {
            [rPrime, gPrime, bPrime] = [x, c, 0];
        } else if (h < 180) {
            [rPrime, gPrime, bPrime] = [0, c, x];
        } else if (h < 240) {
            [rPrime, gPrime, bPrime] = [0, x, c];
        } else if (h < 300) {
            [rPrime, gPrime, bPrime] = [x, 0, c];
        } else {
            [rPrime, gPrime, bPrime] = [c, 0, x];
        }

        return {
            r: Math.round((rPrime + m) * 255),
            g: Math.round((gPrime + m) * 255),
            b: Math.round((bPrime + m) * 255)
        };
    }
}

// Export for use in app.js
if (typeof module !== 'undefined' && module.exports) {
    module.exports = PixelSorter;
}

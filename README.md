# Pixelsort Website

Mobile-friendly web version of the Harpy Pixel Sorter. A touch-optimized pixel sorting creative tool that works directly in your mobile browser.

## Features

- **Camera Capture**: Take pictures directly from your device camera
- **Upload Images**: Load images from your device storage
- **Live Pixel Sorting**: Real-time pixel sorting with instant visual feedback
- **3 Sorting Algorithms**: Horizontal, Vertical, and Diagonal
- **3 Sorting Modes**: Brightness, Black, and White
- **Threshold Control**: Adjust sensitivity of segment breaks (0-255)
- **Hue Shift**: Apply color tinting (0-360 degrees)
- **Save & Iterate**: Download processed images and continue iterating
- **Mobile-Optimized**: Large touch targets, responsive layout, PWA support

## Usage

### Getting Started

1. Open `index.html` in a web browser (works best on mobile)
2. Choose to either:
   - **Take Picture**: Access your device camera
   - **Upload**: Select an image from your device

### Editing

Once an image is loaded, you can:

1. **Adjust Threshold**: Slide to change sorting sensitivity
2. **Adjust Hue**: Slide to apply color tinting
3. **Change Algorithm**: Tap to cycle through Horizontal → Vertical → Diagonal
4. **Change Mode**: Tap to cycle through Brightness → Black → White
5. **Save & Iterate**: Download current result and continue editing
6. **New Image**: Start over with a fresh image

### How It Works

The pixel sorter analyzes your image and sorts pixels based on brightness or color:

- **Threshold**: Higher values create more breaks/segments
- **Horizontal**: Sorts pixels left to right in each row
- **Vertical**: Sorts pixels top to bottom in each column
- **Diagonal**: Sorts pixels along diagonal lines
- **Brightness Mode**: Sorts by luminance
- **Black Mode**: Sorts darkest to lightest
- **White Mode**: Sorts lightest to darkest

## Deployment

### Simple HTTP Server

```bash
# Python 3
python -m http.server 8000

# Node.js
npx http-server

# PHP
php -S localhost:8000
```

Then open `http://localhost:8000/web/` in your browser.

### Production Deployment

Upload the `web/` folder to any static hosting service:
- GitHub Pages
- Netlify
- Vercel
- Firebase Hosting
- AWS S3 + CloudFront

### Progressive Web App (PWA)

The app includes a `manifest.json` for PWA support. Users can install it to their home screen on mobile devices for a native app-like experience.

## Browser Compatibility

- **Modern Browsers**: Chrome, Firefox, Safari, Edge (latest versions)
- **Mobile**: iOS Safari 12+, Chrome Mobile, Samsung Internet
- **Camera**: Requires HTTPS for camera access (except on localhost)

## File Structure

```
web/
  index.html          # Main HTML structure
  styles.css          # Mobile-optimized CSS
  app.js              # Application logic
  pixelsorter.js      # Pixel sorting algorithms
  manifest.json       # PWA manifest
  README.md           # This file
```

## Development

### Testing on Mobile

1. Use browser dev tools mobile emulation
2. Or use a local network:
   ```bash
   python -m http.server 8000
   # Access from phone: http://YOUR_IP:8000/web/
   ```

### Modifying UI

- **Button sizes**: Edit `.btn-large`, `.btn-medium`, `.btn-small` in `styles.css`
- **Colors**: Change `background`, `color` values in CSS
- **Layout**: Modify flexbox properties in `.controls` and `.action-buttons`

### Adding Features

- **New algorithms**: Add to `PixelSorter.algorithms` and implement in `pixelsorter.js`
- **New modes**: Add to `PixelSorter.modes` and update `sortKey()` method
- **UI elements**: Add HTML in `index.html`, style in `styles.css`, wire in `app.js`

## Differences from Rust App

The web version focuses on core functionality:

- ✅ Camera capture
- ✅ Image upload
- ✅ Live pixel sorting
- ✅ All sorting algorithms (Horizontal, Vertical, Diagonal)
- ✅ All sorting modes (Brightness, Black, White)
- ✅ Threshold and Hue controls
- ✅ Save & iterate workflow
- ❌ Crop functionality (omitted as requested)
- ❌ USB export (not applicable for web)
- ❌ Sleep mode (not needed for web)
- ❌ Kiosk mode (not applicable for web)

## License

MIT License - see [LICENSE](../LICENSE) file for details.

## Credits

Web version of [Harpy Pixel Sorter](https://github.com/Vaghabund/Pixelsort), ported from Rust to JavaScript for mobile browser compatibility.

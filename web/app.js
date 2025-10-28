// Main Application
class PixelSortApp {
    constructor() {
        this.sorter = new PixelSorter();
        this.currentAlgorithm = 'Horizontal';
        this.currentMode = 'Brightness';
        this.threshold = 0;
        this.hueShift = 0;
        this.originalImage = null;
        this.processedImage = null;
        this.iterationCount = 0;
        this.sessionId = this.generateSessionId();
        this.cameraStream = null;
        
        this.initUI();
        this.initSplashScreen();
    }

    generateSessionId() {
        const now = new Date();
        const year = now.getFullYear();
        const month = String(now.getMonth() + 1).padStart(2, '0');
        const day = String(now.getDate()).padStart(2, '0');
        const hours = String(now.getHours()).padStart(2, '0');
        const minutes = String(now.getMinutes()).padStart(2, '0');
        const seconds = String(now.getSeconds()).padStart(2, '0');
        return `session_${year}${month}${day}_${hours}${minutes}${seconds}`;
    }

    initSplashScreen() {
        // Hide splash and show app after 2 seconds
        setTimeout(() => {
            document.getElementById('splash').style.display = 'none';
            document.getElementById('app').style.display = 'block';
        }, 2000);
    }

    initUI() {
        // Input phase buttons
        document.getElementById('camera-btn').addEventListener('click', () => this.openCamera());
        document.getElementById('upload-btn').addEventListener('click', () => this.openFileDialog());
        document.getElementById('file-input').addEventListener('change', (e) => this.handleFileSelect(e));

        // Edit phase controls
        document.getElementById('threshold-slider').addEventListener('input', (e) => this.updateThreshold(e));
        document.getElementById('hue-slider').addEventListener('input', (e) => this.updateHue(e));
        document.getElementById('algorithm-btn').addEventListener('click', () => this.cycleAlgorithm());
        document.getElementById('mode-btn').addEventListener('click', () => this.cycleMode());
        document.getElementById('save-btn').addEventListener('click', () => this.saveAndIterate());
        document.getElementById('new-btn').addEventListener('click', () => this.newImage());
    }

    openCamera() {
        if (navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
            const constraints = {
                video: { 
                    facingMode: 'environment',
                    width: { ideal: 1920 },
                    height: { ideal: 1080 }
                }
            };

            navigator.mediaDevices.getUserMedia(constraints)
                .then(stream => {
                    this.cameraStream = stream;
                    const video = document.getElementById('camera-preview');
                    video.srcObject = stream;
                    video.style.display = 'block';
                    
                    // Add capture button overlay
                    this.showCaptureButton(video);
                })
                .catch(err => {
                    console.error('Camera access denied:', err);
                    this.showStatus('Camera not available');
                });
        } else {
            this.showStatus('Camera not supported');
        }
    }

    showCaptureButton(video) {
        // Create capture button
        const captureBtn = document.createElement('button');
        captureBtn.className = 'btn btn-large';
        captureBtn.style.position = 'fixed';
        captureBtn.style.bottom = '40px';
        captureBtn.style.left = '50%';
        captureBtn.style.transform = 'translateX(-50%)';
        captureBtn.style.zIndex = '20';
        captureBtn.innerHTML = `
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <circle cx="12" cy="12" r="6" fill="currentColor"></circle>
            </svg>
            <span>Capture</span>
        `;
        
        captureBtn.addEventListener('click', () => {
            this.captureFromCamera(video);
            video.style.display = 'none';
            if (this.cameraStream) {
                this.cameraStream.getTracks().forEach(track => track.stop());
                this.cameraStream = null;
            }
            captureBtn.remove();
        });
        
        document.body.appendChild(captureBtn);
    }

    captureFromCamera(video) {
        const canvas = document.createElement('canvas');
        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;
        const ctx = canvas.getContext('2d');
        ctx.drawImage(video, 0, 0);
        
        canvas.toBlob(blob => {
            const file = new File([blob], 'camera-capture.jpg', { type: 'image/jpeg' });
            this.loadImageFromFile(file);
        }, 'image/jpeg', 0.95);
    }

    openFileDialog() {
        document.getElementById('file-input').click();
    }

    handleFileSelect(event) {
        const file = event.target.files[0];
        if (file) {
            this.loadImageFromFile(file);
        }
    }

    loadImageFromFile(file) {
        const reader = new FileReader();
        reader.onload = (e) => {
            const img = new Image();
            img.onload = () => {
                this.originalImage = img;
                this.processImage();
                this.switchToEditPhase();
            };
            img.src = e.target.result;
        };
        reader.readAsDataURL(file);
    }

    processImage() {
        if (!this.originalImage) return;

        const canvas = document.getElementById('display-canvas');
        const ctx = canvas.getContext('2d');
        
        // Set canvas size to match image
        canvas.width = this.originalImage.width;
        canvas.height = this.originalImage.height;
        
        // Draw original image
        ctx.drawImage(this.originalImage, 0, 0);
        
        // Get image data
        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        
        // Apply pixel sorting
        const params = {
            threshold: this.threshold,
            hueShift: this.hueShift,
            sortMode: this.currentMode
        };
        
        const sortedData = this.sorter.sortPixels(imageData, this.currentAlgorithm, params);
        
        // Put sorted data back
        ctx.putImageData(sortedData, 0, 0);
        
        this.processedImage = canvas;
    }

    switchToEditPhase() {
        document.getElementById('input-phase').style.display = 'none';
        document.getElementById('edit-phase').style.display = 'flex';
    }

    switchToInputPhase() {
        document.getElementById('input-phase').style.display = 'flex';
        document.getElementById('edit-phase').style.display = 'none';
        this.originalImage = null;
        this.processedImage = null;
        this.iterationCount = 0;
    }

    updateThreshold(event) {
        this.threshold = parseInt(event.target.value);
        document.getElementById('threshold-value').textContent = this.threshold;
        this.processImage();
    }

    updateHue(event) {
        this.hueShift = parseInt(event.target.value);
        document.getElementById('hue-value').textContent = this.hueShift;
        this.processImage();
    }

    cycleAlgorithm() {
        const algorithms = this.sorter.algorithms;
        const currentIndex = algorithms.indexOf(this.currentAlgorithm);
        this.currentAlgorithm = algorithms[(currentIndex + 1) % algorithms.length];
        document.getElementById('algorithm-text').textContent = this.currentAlgorithm;
        this.processImage();
    }

    cycleMode() {
        const modes = this.sorter.modes;
        const currentIndex = modes.indexOf(this.currentMode);
        this.currentMode = modes[(currentIndex + 1) % modes.length];
        document.getElementById('mode-text').textContent = this.currentMode;
        this.processImage();
    }

    saveAndIterate() {
        if (!this.processedImage) return;

        this.iterationCount++;
        const canvas = document.getElementById('display-canvas');
        
        // Generate filename
        const filename = `edit_${String(this.iterationCount).padStart(3, '0')}_${this.currentAlgorithm.toLowerCase()}.png`;
        
        // Download image
        canvas.toBlob(blob => {
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = filename;
            a.click();
            URL.revokeObjectURL(url);
            
            // Show success message
            this.showStatus(`Saved: ${filename}`);
            
            // Update original image to current processed image for iteration
            const img = new Image();
            img.onload = () => {
                this.originalImage = img;
            };
            img.src = canvas.toDataURL();
        });
    }

    newImage() {
        if (confirm('Start over with a new image?')) {
            // Reset state
            this.threshold = 0;
            this.hueShift = 0;
            this.currentAlgorithm = 'Horizontal';
            this.currentMode = 'Brightness';
            
            // Reset UI
            document.getElementById('threshold-slider').value = 0;
            document.getElementById('threshold-value').textContent = '0';
            document.getElementById('hue-slider').value = 0;
            document.getElementById('hue-value').textContent = '0';
            document.getElementById('algorithm-text').textContent = 'Horizontal';
            document.getElementById('mode-text').textContent = 'Brightness';
            
            // Switch back to input phase
            this.switchToInputPhase();
        }
    }

    showStatus(message) {
        const statusEl = document.getElementById('status-message');
        statusEl.textContent = message;
        statusEl.classList.add('show');
        
        setTimeout(() => {
            statusEl.classList.remove('show');
        }, 3000);
    }
}

// Initialize app when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new PixelSortApp();
});

const fs = require('fs');
const path = require('path');

const outDir = path.join(__dirname, 'dist');
if (!fs.existsSync(outDir)) {
  fs.mkdirSync(outDir, { recursive: true });
}

// Copy core files
const files = ['index.html', 'styles.css', 'app.js'];
files.forEach((f) => {
  if (fs.existsSync(path.join(__dirname, f))) {
    fs.copyFileSync(path.join(__dirname, f), path.join(outDir, f));
  }
});

// Copy assets folder
const assetsDir = path.join(__dirname, 'assets');
const outAssets = path.join(outDir, 'assets');
if (fs.existsSync(assetsDir)) {
  if (!fs.existsSync(outAssets)) {
    fs.mkdirSync(outAssets, { recursive: true });
  }
  fs.readdirSync(assetsDir).forEach((file) => {
    fs.copyFileSync(path.join(assetsDir, file), path.join(outAssets, file));
  });
}

console.log('Build completed: generated /dist directory');

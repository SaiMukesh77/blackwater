/**
 * Blackwater — Client-Side Binary Photo Extraction & Conversion Engine
 * 100% in-browser, private, offline processing for RDR2 PRDR3* files.
 */

(function () {
  'use strict';

  // State
  let extractedPhotos = [];
  let totalScannedFiles = 0;
  let targetFormat = 'jpeg'; // 'jpeg' or 'png'
  let currentActivePhoto = null;

  // DOM Elements
  const dropzone = document.getElementById('dropzone');
  const fileInput = document.getElementById('file-input');
  const folderInput = document.getElementById('folder-input');
  const btnSelectFiles = document.getElementById('btn-select-files');
  const btnSelectFolder = document.getElementById('btn-select-folder');
  const btnLoadDemo = document.getElementById('btn-load-demo');
  const statsPanel = document.getElementById('stats-panel');
  const statScanned = document.getElementById('stat-scanned');
  const statExtracted = document.getElementById('stat-extracted');
  const statSize = document.getElementById('stat-size');
  const btnDownloadZip = document.getElementById('btn-download-zip');
  const btnClear = document.getElementById('btn-clear');
  const galleryHeader = document.getElementById('gallery-header');
  const galleryCountBadge = document.getElementById('gallery-count-badge');
  const galleryGrid = document.getElementById('gallery-grid');
  const formatToggles = document.querySelectorAll('.format-toggle .toggle-btn');
  const btnCopyPath = document.getElementById('btn-copy-path');
  const pathText = document.getElementById('path-text');
  const lightbox = document.getElementById('lightbox');
  const lightboxImg = document.getElementById('lightbox-img');
  const lightboxTitle = document.getElementById('lightbox-title');
  const lightboxInfo = document.getElementById('lightbox-info');
  const btnLightboxDownload = document.getElementById('btn-lightbox-download');
  const btnCloseLightbox = document.getElementById('btn-close-lightbox');
  const toastContainer = document.getElementById('toast-container');

  // Initialize Event Listeners
  function init() {
    // File Selection
    btnSelectFiles.addEventListener('click', () => fileInput.click());
    btnSelectFolder.addEventListener('click', () => folderInput.click());
    fileInput.addEventListener('change', handleFileInput);
    folderInput.addEventListener('change', handleFileInput);

    // Drag & Drop
    dropzone.addEventListener('dragover', (e) => {
      e.preventDefault();
      dropzone.classList.add('drag-over');
    });

    dropzone.addEventListener('dragleave', () => {
      dropzone.classList.remove('drag-over');
    });

    dropzone.addEventListener('drop', handleDrop);

    // Format Toggle
    formatToggles.forEach((btn) => {
      btn.addEventListener('click', () => {
        formatToggles.forEach((b) => b.classList.remove('active'));
        btn.classList.add('active');
        targetFormat = btn.getAttribute('data-format');
        showToast(`Output format set to ${targetFormat.toUpperCase()}`);
      });
    });

    // Batch Actions
    btnDownloadZip.addEventListener('click', handleDownloadZip);
    btnClear.addEventListener('click', handleClear);

    // Demo Loader
    btnLoadDemo.addEventListener('click', handleLoadDemo);

    // Copy Path
    btnCopyPath.addEventListener('click', handleCopyPath);

    // Lightbox
    btnCloseLightbox.addEventListener('click', () => lightbox.classList.remove('active'));
    lightbox.addEventListener('click', (e) => {
      if (e.target === lightbox) lightbox.classList.remove('active');
    });
    btnLightboxDownload.addEventListener('click', () => {
      if (currentActivePhoto) {
        downloadSinglePhoto(currentActivePhoto, targetFormat);
      }
    });

    // Keyboard navigation
    window.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && lightbox.classList.contains('active')) {
        lightbox.classList.remove('active');
      }
    });
  }

  // Toast Notification
  function showToast(message, type = 'info') {
    const toast = document.createElement('div');
    toast.className = 'toast';
    toast.innerHTML = `
      <svg width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
      <span>${escapeHtml(message)}</span>
    `;
    toastContainer.appendChild(toast);
    setTimeout(() => {
      toast.style.opacity = '0';
      toast.style.transform = 'translateY(10px)';
      toast.style.transition = 'all 0.3s ease';
      setTimeout(() => toast.remove(), 300);
    }, 3200);
  }

  function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  // Copy Save Directory Path
  function handleCopyPath() {
    const cleanPath = '%USERPROFILE%\\Documents\\Rockstar Games\\Red Dead Redemption 2\\Profiles\\';
    navigator.clipboard.writeText(cleanPath).then(() => {
      showToast('Path copied to clipboard!');
      const copyText = document.getElementById('copy-text');
      copyText.textContent = 'Copied!';
      setTimeout(() => (copyText.textContent = 'Copy'), 2000);
    });
  }

  // File Input Handler
  function handleFileInput(e) {
    const files = Array.from(e.target.files);
    if (files.length > 0) {
      processFiles(files);
    }
  }

  // Drag & Drop Handler
  async function handleDrop(e) {
    e.preventDefault();
    dropzone.classList.remove('drag-over');

    const items = e.dataTransfer.items;
    const files = [];

    if (items) {
      for (let i = 0; i < items.length; i++) {
        const item = items[i].webkitGetAsEntry ? items[i].webkitGetAsEntry() : null;
        if (item) {
          await scanEntry(item, files);
        } else if (items[i].kind === 'file') {
          const file = items[i].getAsFile();
          if (file) files.push(file);
        }
      }
    } else if (e.dataTransfer.files) {
      files.push(...Array.from(e.dataTransfer.files));
    }

    if (files.length > 0) {
      processFiles(files);
    }
  }

  // Recursive Directory Scanner
  function scanEntry(entry, fileList) {
    return new Promise((resolve) => {
      if (entry.isFile) {
        entry.file((file) => {
          fileList.push(file);
          resolve();
        });
      } else if (entry.isDirectory) {
        const dirReader = entry.createReader();
        dirReader.readEntries(async (entries) => {
          for (const ent of entries) {
            await scanEntry(ent, fileList);
          }
          resolve();
        });
      } else {
        resolve();
      }
    });
  }

  // Binary Scanner & Extractor Engine
  async function processFiles(files) {
    totalScannedFiles += files.length;
    let extractedInBatch = 0;

    for (const file of files) {
      try {
        const arrayBuffer = await file.arrayBuffer();
        const bytes = new Uint8Array(arrayBuffer);
        const result = scanForJpeg(bytes);

        if (result) {
          const jpegBlob = new Blob([result.jpegBytes], { type: 'image/jpeg' });
          const objectUrl = URL.createObjectURL(jpegBlob);

          const photoObj = {
            id: 'photo_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6),
            originalName: file.name,
            outputBaseName: file.name.replace(/^PRDR\d*_?/, 'Photo_'),
            jpegBytes: result.jpegBytes,
            blobUrl: objectUrl,
            width: result.width || 1920,
            height: result.height || 1080,
            fileSize: result.jpegBytes.byteLength,
            sourceSize: file.size,
            offset: result.offset,
          };

          extractedPhotos.push(photoObj);
          extractedInBatch++;
          renderPhotoCard(photoObj);
        }
      } catch (err) {
        console.error('Error scanning file:', file.name, err);
      }
    }

    updateDashboard();

    if (extractedInBatch > 0) {
      showToast(`Extracted ${extractedInBatch} RDR2 photo${extractedInBatch > 1 ? 's' : ''}!`);
    } else {
      showToast('No embedded JPEG found in chosen files. Ensure you selected PRDR3* files.', 'warn');
    }
  }

  /**
   * Scans a byte array for embedded JPEG headers (SOI: FF D8 FF) and finds exact EOI (FF D9).
   */
  function scanForJpeg(bytes) {
    const len = bytes.length;
    let soiOffset = -1;

    // 1. Search for SOI: 0xFF, 0xD8, 0xFF
    for (let i = 0; i < len - 2; i++) {
      if (bytes[i] === 0xff && bytes[i + 1] === 0xd8 && bytes[i + 2] === 0xff) {
        soiOffset = i;
        break;
      }
    }

    if (soiOffset === -1) {
      return null;
    }

    // 2. Search for EOI: 0xFF, 0xD9 from soiOffset forward
    let eoiOffset = -1;
    for (let i = soiOffset + 2; i < len - 1; i++) {
      if (bytes[i] === 0xff && bytes[i + 1] === 0xd9) {
        eoiOffset = i + 2; // Include 0xD9
      }
    }

    if (eoiOffset === -1 || eoiOffset <= soiOffset) {
      // Fallback: take till end of stream if valid
      eoiOffset = len;
    }

    const jpegBytes = bytes.slice(soiOffset, eoiOffset);

    // 3. Extract dimensions from SOF marker
    const dimensions = extractDimensions(jpegBytes);

    return {
      offset: soiOffset,
      length: eoiOffset - soiOffset,
      jpegBytes,
      width: dimensions ? dimensions.width : null,
      height: dimensions ? dimensions.height : null,
    };
  }

  /**
   * Extracts Width & Height from SOF segment
   */
  function extractDimensions(bytes) {
    for (let i = 0; i < bytes.length - 8; i++) {
      if (bytes[i] === 0xff) {
        const marker = bytes[i + 1];
        // SOF0 (0xC0), SOF1 (0xC1), SOF2 (0xC2), etc.
        if (
          marker === 0xc0 ||
          marker === 0xc1 ||
          marker === 0xc2 ||
          marker === 0xc3 ||
          marker === 0xc5 ||
          marker === 0xc6 ||
          marker === 0xc7 ||
          marker === 0xc9 ||
          marker === 0xca ||
          marker === 0xcb ||
          marker === 0xcd ||
          marker === 0xce ||
          marker === 0xcf
        ) {
          const height = (bytes[i + 5] << 8) | bytes[i + 6];
          const width = (bytes[i + 7] << 8) | bytes[i + 8];
          if (width > 0 && height > 0) {
            return { width, height };
          }
        }
      }
    }
    return null;
  }

  // Render a Single Extracted Photo in Gallery Grid
  function renderPhotoCard(photo) {
    const card = document.createElement('div');
    card.className = 'photo-card';
    card.id = photo.id;

    const sizeFormatted = (photo.fileSize / 1024).toFixed(1) + ' KB';
    const dimText = `${photo.width} × ${photo.height}`;

    card.innerHTML = `
      <div class="photo-thumb-wrap" title="Click to enlarge">
        <img src="${photo.blobUrl}" alt="${escapeHtml(photo.originalName)}" class="photo-thumb" loading="lazy">
        <span class="photo-badge">JPEG</span>
        <div class="photo-overlay">
          <svg width="32" height="32" fill="none" stroke="#fff" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v6m3-3H7"/></svg>
        </div>
      </div>
      <div class="photo-info">
        <div class="photo-name">${escapeHtml(photo.originalName)}</div>
        <div class="photo-meta-row">
          <span>${dimText}</span>
          <span>${sizeFormatted}</span>
        </div>
        <div class="photo-card-actions">
          <button class="btn btn-gold btn-dl-jpg">JPEG</button>
          <button class="btn btn-glass btn-dl-png">PNG</button>
        </div>
      </div>
    `;

    // Click to Open Lightbox
    card.querySelector('.photo-thumb-wrap').addEventListener('click', () => {
      openLightbox(photo);
    });

    // Quick single downloads
    card.querySelector('.btn-dl-jpg').addEventListener('click', (e) => {
      e.stopPropagation();
      downloadSinglePhoto(photo, 'jpeg');
    });

    card.querySelector('.btn-dl-png').addEventListener('click', (e) => {
      e.stopPropagation();
      downloadSinglePhoto(photo, 'png');
    });

    galleryGrid.appendChild(card);
  }

  // Open Lightbox
  function openLightbox(photo) {
    currentActivePhoto = photo;
    lightboxImg.src = photo.blobUrl;
    lightboxTitle.textContent = photo.originalName;
    lightboxInfo.textContent = `${photo.width} × ${photo.height} • ${(photo.fileSize / 1024).toFixed(1)} KB (Offset: 0x${photo.offset.toString(16).toUpperCase()})`;
    lightbox.classList.add('active');
  }

  // Single Photo Download (JPEG or converted PNG via Canvas)
  async function downloadSinglePhoto(photo, format) {
    if (format === 'jpeg') {
      const a = document.createElement('a');
      a.href = photo.blobUrl;
      a.download = `${photo.outputBaseName}.jpg`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      showToast(`Downloaded ${photo.outputBaseName}.jpg`);
    } else {
      showToast('Converting to lossless PNG...');
      const pngBlob = await convertJpegToPng(photo.blobUrl);
      const url = URL.createObjectURL(pngBlob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${photo.outputBaseName}.png`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      showToast(`Downloaded ${photo.outputBaseName}.png`);
    }
  }

  // Convert JPEG blob url to PNG blob using HTML5 Canvas
  function convertJpegToPng(blobUrl) {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement('canvas');
        canvas.width = img.naturalWidth;
        canvas.height = img.naturalHeight;
        const ctx = canvas.getContext('2d');
        ctx.drawImage(img, 0, 0);
        canvas.toBlob((blob) => resolve(blob), 'image/png');
      };
      img.src = blobUrl;
    });
  }

  // Batch ZIP Download
  async function handleDownloadZip() {
    if (extractedPhotos.length === 0) return;

    if (typeof JSZip === 'undefined') {
      showToast('JSZip library loading, please wait a second...', 'warn');
      return;
    }

    const zip = new JSZip();
    const folderName = `Blackwater_RDR2_Photos_${new Date().toISOString().slice(0, 10)}`;
    const imgFolder = zip.folder(folderName);

    showToast(`Bundling ${extractedPhotos.length} photos as ${targetFormat.toUpperCase()}...`);
    btnDownloadZip.disabled = true;
    btnDownloadZip.innerHTML = '<span>Zipping...</span>';

    try {
      for (let i = 0; i < extractedPhotos.length; i++) {
        const p = extractedPhotos[i];
        const num = String(i + 1).padStart(3, '0');
        const filename = `RDR2_Photo_${num}.${targetFormat === 'jpeg' ? 'jpg' : 'png'}`;

        if (targetFormat === 'jpeg') {
          imgFolder.file(filename, p.jpegBytes);
        } else {
          const pngBlob = await convertJpegToPng(p.blobUrl);
          const pngBuffer = await pngBlob.arrayBuffer();
          imgFolder.file(filename, pngBuffer);
        }
      }

      const content = await zip.generateAsync({ type: 'blob' });
      const url = URL.createObjectURL(content);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${folderName}.zip`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);

      showToast(`Downloaded ${folderName}.zip!`);
    } catch (err) {
      console.error('ZIP generation error:', err);
      showToast('Failed to create ZIP archive', 'error');
    } finally {
      btnDownloadZip.disabled = false;
      btnDownloadZip.innerHTML = `
        <svg width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
        <span>Download All (ZIP)</span>
      `;
    }
  }

  // Update UI Dashboard Stats
  function updateDashboard() {
    if (extractedPhotos.length > 0 || totalScannedFiles > 0) {
      statsPanel.style.display = 'flex';
      galleryHeader.style.display = 'flex';
    } else {
      statsPanel.style.display = 'none';
      galleryHeader.style.display = 'none';
    }

    statScanned.textContent = totalScannedFiles;
    statExtracted.textContent = extractedPhotos.length;

    const totalBytes = extractedPhotos.reduce((acc, p) => acc + p.fileSize, 0);
    if (totalBytes > 1024 * 1024) {
      statSize.textContent = (totalBytes / (1024 * 1024)).toFixed(2) + ' MB';
    } else {
      statSize.textContent = (totalBytes / 1024).toFixed(1) + ' KB';
    }

    galleryCountBadge.textContent = `${extractedPhotos.length} photo${extractedPhotos.length !== 1 ? 's' : ''}`;
    btnDownloadZip.disabled = extractedPhotos.length === 0;
  }

  // Clear All
  function handleClear() {
    extractedPhotos.forEach((p) => URL.revokeObjectURL(p.blobUrl));
    extractedPhotos = [];
    totalScannedFiles = 0;
    galleryGrid.innerHTML = '';
    updateDashboard();
    fileInput.value = '';
    folderInput.value = '';
    showToast('Cleared all extracted photos');
  }

  // Load Demo Sample Generator
  async function handleLoadDemo() {
    showToast('Generating demo RDR2 save file stream...');
    try {
      // Fetch the hero image as sample JPEG
      const response = await fetch('assets/hero_bg.jpg');
      const sampleJpegBuffer = await response.arrayBuffer();

      // Synthesize a binary RDR2 PRDR3 format with Rockstar header padding before SOI
      const headerPadding = new Uint8Array([
        0x52, 0x44, 0x52, 0x32, 0x01, 0x00, 0x00, 0x00, 0x53, 0x41, 0x56, 0x45, 0x00, 0x00, 0x00, 0x00,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      ]);

      const totalSize = headerPadding.length + sampleJpegBuffer.byteLength + 16;
      const rdr2Save = new Uint8Array(totalSize);
      rdr2Save.set(headerPadding, 0);
      rdr2Save.set(new Uint8Array(sampleJpegBuffer), headerPadding.length);

      // Trailing footer
      rdr2Save.set([0x00, 0x00, 0x45, 0x4e, 0x44, 0x5f, 0x53, 0x41, 0x56, 0x45], headerPadding.length + sampleJpegBuffer.byteLength);

      // Create synthetic File object
      const demoFile = new File([rdr2Save.buffer], 'PRDR3128919023_1', { type: 'application/octet-stream' });
      await processFiles([demoFile]);

      // Scroll to extractor
      document.getElementById('extractor').scrollIntoView({ behavior: 'smooth' });
    } catch (err) {
      console.error('Demo error:', err);
      showToast('Could not load demo asset', 'error');
    }
  }

  // Run on DOM ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();

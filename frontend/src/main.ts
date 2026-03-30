import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';

window.onerror = (message, source, lineno, colno, error) => {
  console.error('[JS ERROR]', message, source, lineno, colno, error);
};

window.onunhandledrejection = (event) => {
  console.error('[UNHANDLED REJECTION]', event.reason);
};

console.log('[INFO] Frontend script loaded');

type Operation = 'convert' | 'compress' | 'resize' | 'trim' | 'extract_audio' | 'gif' | 'rotate' | 'watermark';

interface ConvertParams {
  output_format: string;
}

interface CompressParams {
  crf: number;
}

interface ResizeParams {
  width: number;
  height: number;
}

interface TrimParams {
  start_time: number;
  duration: number;
}

interface GifParams {
  fps: number;
  scale: number;
}

interface RotateParams {
  angle: number;
}

interface WatermarkParams {
  text: string;
}

type OperationParams = ConvertParams | CompressParams | ResizeParams | TrimParams | GifParams | RotateParams | WatermarkParams | Record<string, unknown>;

interface ProcessRequest {
  operation: Operation;
  params: OperationParams;
  file_path: string;
}

interface JobStatus {
  status: 'processing' | 'completed' | 'failed';
  progress: number;
  error?: string;
}

interface ProcessResponse {
  success: boolean;
  job_id?: string;
  error?: string;
  result_path?: string;
}

const operationNames: Record<Operation, string> = {
  convert: 'Convert Format',
  compress: 'Compress Video',
  resize: 'Resize Video',
  trim: 'Trim Video',
  extract_audio: 'Extract Audio',
  gif: 'Create GIF',
  rotate: 'Rotate Video',
  watermark: 'Add Watermark',
};

document.addEventListener('DOMContentLoaded', () => {
  const step1 = document.getElementById('step1') as HTMLDivElement;
  const step2 = document.getElementById('step2') as HTMLDivElement;
  const step3 = document.getElementById('step3') as HTMLDivElement;
  const step4 = document.getElementById('step4') as HTMLDivElement;
  const startOverButton = document.getElementById('startOverButton') as HTMLButtonElement;
  const selectFileButton = document.getElementById('selectFileButton') as HTMLButtonElement;
  const fileStatus = document.getElementById('fileStatus') as HTMLDivElement;
  const settingsContainer = document.getElementById('settingsContainer') as HTMLDivElement;
  const processButton = document.getElementById('processButton') as HTMLButtonElement;
  const backToStep1 = document.getElementById('backToStep1') as HTMLButtonElement;
  const backToStep2 = document.getElementById('backToStep2') as HTMLButtonElement;
  const progressBar = document.getElementById('progressBar') as HTMLDivElement;
  const progressText = document.getElementById('progressText') as HTMLParagraphElement;
  const statusMessage = document.getElementById('statusMessage') as HTMLDivElement;
  const selectedOperationText = document.getElementById('selectedOperation') as HTMLParagraphElement;

  let currentFilePath: string = '';
  let selectedOperation: Operation = '' as Operation;
  let operationParams: OperationParams = {};
  let currentJobId: string = '';
  let unlistenProgress: (() => void) | null = null;

  // Step 1: Operation selection
  document.querySelectorAll<HTMLButtonElement>('.operation-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      document.querySelectorAll<HTMLButtonElement>('.operation-btn').forEach((b) => b.classList.remove('selected'));
      btn.classList.add('selected');

      selectedOperation = btn.getAttribute('data-operation') as Operation;
      selectedOperationText.textContent = 'Selected: ' + operationNames[selectedOperation];

      setTimeout(() => {
        step1.style.display = 'none';
        step2.style.display = 'block';
      }, 300);
    });
  });

  // Back to step 1
  backToStep1.addEventListener('click', () => {
    step2.style.display = 'none';
    step1.style.display = 'block';
  });

  // Step 2: File selection using Tauri dialog
  selectFileButton.addEventListener('click', async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Media Files',
            extensions: ['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'mpg', 'mpeg', 'mp3', 'wav', 'flac', 'aac', 'ogg', 'wma', 'jpg', 'jpeg', 'png', 'gif', 'bmp', 'tiff', 'webp'],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        currentFilePath = selected;
        const fileName = selected.split(/[/\\]/).pop() || selected;
        fileStatus.innerHTML = '<span class="success">Selected: ' + fileName + '</span>';

        loadSettings();
        setTimeout(() => {
          step2.style.display = 'none';
          step3.style.display = 'block';
        }, 500);
      } else {
        fileStatus.innerHTML = '<div class="error">No file selected</div>';
      }
    } catch (error) {
      fileStatus.innerHTML = '<div class="error">Error: ' + String(error) + '</div>';
    }
  });

  // Back to step 2
  backToStep2.addEventListener('click', () => {
    step3.style.display = 'none';
    step2.style.display = 'block';
  });

  // Load settings based on selected operation
  function loadSettings(): void {
    let html = '';

    switch (selectedOperation) {
      case 'convert':
        html = `
          <div class="setting-group">
            <label for="outputFormat">Output Format</label>
            <select id="outputFormat">
              <option value="mp4">MP4</option>
              <option value="avi">AVI</option>
              <option value="mkv">MKV</option>
              <option value="mov">MOV</option>
              <option value="webm">WEBM</option>
            </select>
          </div>
        `;
        break;
      case 'compress':
        html = `
          <div class="setting-group">
            <label for="crf">Quality (CRF): <span id="crfValue">23</span></label>
            <input type="range" id="crf" min="18" max="28" value="23">
          </div>
        `;
        break;
      case 'resize':
        html = `
          <div class="setting-group">
            <label for="width">Width (px)</label>
            <input type="number" id="width" value="1280" min="100">
          </div>
          <div class="setting-group">
            <label for="height">Height (px)</label>
            <input type="number" id="height" value="720" min="100">
          </div>
        `;
        break;
      case 'trim':
        html = `
          <div class="setting-group">
            <label for="startTime">Start Time (seconds)</label>
            <input type="number" id="startTime" value="0" min="0" step="0.1">
          </div>
          <div class="setting-group">
            <label for="duration">Duration (seconds)</label>
            <input type="number" id="duration" value="10" min="0.1" step="0.1">
          </div>
        `;
        break;
      case 'extract_audio':
        html = `<p>Audio will be extracted as MP3.</p>`;
        break;
      case 'gif':
        html = `
          <div class="setting-group">
            <label for="gifFps">FPS</label>
            <input type="number" id="gifFps" value="15" min="1" max="60">
          </div>
          <div class="setting-group">
            <label for="gifScale">Width (px)</label>
            <input type="number" id="gifScale" value="320" min="50">
          </div>
        `;
        break;
      case 'rotate':
        html = `
          <div class="setting-group">
            <label for="rotateAngle">Rotation</label>
            <select id="rotateAngle">
              <option value="90">90° Clockwise</option>
              <option value="180">180°</option>
              <option value="270">90° Counter-clockwise</option>
            </select>
          </div>
        `;
        break;
      case 'watermark':
        html = `
          <div class="setting-group">
            <label for="watermarkText">Watermark Text</label>
            <input type="text" id="watermarkText" value="FFmpeg Studio">
          </div>
        `;
        break;
    }

    settingsContainer.innerHTML = html;

    // Add event listeners for params collection
    document.querySelectorAll<HTMLInputElement | HTMLSelectElement>('#settingsContainer input, #settingsContainer select').forEach((input) => {
      input.addEventListener('change', collectParams);
      input.addEventListener('input', collectParams);
    });

    // Special handling for CRF range
    const crfInput = document.getElementById('crf') as HTMLInputElement;
    const crfValue = document.getElementById('crfValue');
    if (crfInput && crfValue) {
      crfInput.addEventListener('input', () => {
        crfValue.textContent = crfInput.value;
        collectParams();
      });
    }

    collectParams();
  }

  function collectParams(): void {
    operationParams = {};

    switch (selectedOperation) {
      case 'convert':
        operationParams = { output_format: (document.getElementById('outputFormat') as HTMLSelectElement)?.value || 'mp4' };
        break;
      case 'compress':
        operationParams = { crf: parseInt((document.getElementById('crf') as HTMLInputElement)?.value || '23', 10) };
        break;
      case 'gif':
        operationParams = {
          fps: parseInt((document.getElementById('gifFps') as HTMLInputElement)?.value || '15', 10),
          scale: parseInt((document.getElementById('gifScale') as HTMLInputElement)?.value || '320', 10),
        };
        break;
      case 'resize':
        operationParams = {
          width: parseInt((document.getElementById('width') as HTMLInputElement)?.value || '1280', 10),
          height: parseInt((document.getElementById('height') as HTMLInputElement)?.value || '720', 10),
        };
        break;
      case 'trim':
        operationParams = {
          start_time: parseFloat((document.getElementById('startTime') as HTMLInputElement)?.value || '0'),
          duration: parseFloat((document.getElementById('duration') as HTMLInputElement)?.value || '10'),
        };
        break;
      case 'rotate':
        operationParams = { angle: parseInt((document.getElementById('rotateAngle') as HTMLSelectElement)?.value || '90', 10) };
        break;
      case 'watermark':
        operationParams = { text: (document.getElementById('watermarkText') as HTMLInputElement)?.value || 'FFmpeg Studio' };
        break;
    }
  }

  // Process button
  processButton.addEventListener('click', async () => {
    if (!selectedOperation || !currentFilePath) {
      alert('Please select an operation and a file');
      return;
    }

    collectParams();

    step3.style.display = 'none';
    step4.style.display = 'block';

    progressBar.style.width = '0%';
    progressText.textContent = '0%';
    statusMessage.textContent = 'Starting processing...';
    statusMessage.className = '';

    try {
      const request: ProcessRequest = {
        operation: selectedOperation,
        params: operationParams,
        file_path: currentFilePath,
      };

      const result = await invoke<ProcessResponse>('process_video', { request });

      if (!result.success) {
        throw new Error(result.error || 'Failed to start processing');
      }

      currentJobId = result.job_id || '';

      // Listen for progress events from Rust backend
      unlistenProgress = await listen<JobStatus>('progress', (event) => {
        const statusData = event.payload;

        progressBar.style.width = statusData.progress + '%';
        progressText.textContent = statusData.progress + '%';

        if (statusData.status === 'processing') {
          statusMessage.textContent = 'Processing...';
        } else if (statusData.status === 'completed') {
          statusMessage.textContent = 'Complete!';
          statusMessage.className = 'success';
          progressBar.style.width = '100%';
          progressText.textContent = '100%';
          startOverButton.style.display = 'block';

          // Trigger download
          if (result.result_path) {
            saveResultFile(result.result_path);
          }
        } else if (statusData.status === 'failed') {
          statusMessage.textContent = 'Failed: ' + (statusData.error || 'Unknown error');
          statusMessage.className = 'error';
          startOverButton.style.display = 'block';
        }
      });

      // Also poll for status as fallback
      pollStatus();
    } catch (error) {
      statusMessage.textContent = 'Error: ' + String(error);
      statusMessage.className = 'error';
      startOverButton.style.display = 'block';
    }
  });

  // Poll status as backup to event listener
  async function pollStatus(): Promise<void> {
    if (!currentJobId) return;

    try {
      const status = await invoke<JobStatus>('get_status', { jobId: currentJobId });

      if (status.status === 'completed') {
        progressBar.style.width = '100%';
        progressText.textContent = '100%';
        statusMessage.textContent = 'Complete!';
        statusMessage.className = 'success';
        startOverButton.style.display = 'block';
      } else if (status.status === 'failed') {
        statusMessage.textContent = 'Failed: ' + (status.error || 'Unknown error');
        statusMessage.className = 'error';
        startOverButton.style.display = 'block';
      } else {
        progressBar.style.width = status.progress + '%';
        progressText.textContent = status.progress + '%';
        setTimeout(pollStatus, 500);
      }
    } catch (error) {
      console.error('Status poll error:', error);
      setTimeout(pollStatus, 1000);
    }
  }

  // Save result file
  async function saveResultFile(resultPath: string): Promise<void> {
    try {
      const savePath = await save({
        filters: [
          {
            name: 'All Files',
            extensions: ['*'],
          },
        ],
        defaultPath: resultPath.split(/[/\\]/).pop(),
      });

      if (savePath) {
        await invoke('save_result', { sourcePath: resultPath, destPath: savePath });
        statusMessage.textContent = 'Saved to: ' + savePath;
      }
    } catch (error) {
      console.error('Save error:', error);
      statusMessage.textContent = 'Processing complete. File at: ' + resultPath;
    }
  }

  // Start Over button
  startOverButton.addEventListener('click', async () => {
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = null;
    }
    currentFilePath = '';
    selectedOperation = '' as Operation;
    currentJobId = '';
    step4.style.display = 'none';
    step1.style.display = 'block';
    document.querySelectorAll<HTMLButtonElement>('.operation-btn').forEach((b) => b.classList.remove('selected'));
    selectedOperationText.textContent = '';
    fileStatus.innerHTML = '';
    settingsContainer.innerHTML = '';
  });
});

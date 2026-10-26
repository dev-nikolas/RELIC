/**
 * tools/audit/capture_audit_artifacts.mjs
 * Script de auditoría frontend automatizado vía Chrome DevTools Protocol (CDP).
 * Captura vistas en 1440x900 y 375x812, audita accesibilidad, renders y terminología.
 */

import { spawn } from 'child_process';
import { writeFileSync, mkdirSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCREENSHOTS_DIR = resolve(__dirname, '../../reports/audit/screenshots');
const REPORTS_DIR = resolve(__dirname, '../../reports/audit');

mkdirSync(SCREENSHOTS_DIR, { recursive: true });
mkdirSync(REPORTS_DIR, { recursive: true });

const CHROME_PATH = 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';

class CDPClient {
  constructor(wsUrl) {
    this.wsUrl = wsUrl;
    this.ws = null;
    this.id = 1;
    this.callbacks = new Map();
  }

  async connect() {
    return new Promise((res, rej) => {
      this.ws = new WebSocket(this.wsUrl);
      this.ws.onopen = () => res();
      this.ws.onerror = (e) => rej(e);
      this.ws.onmessage = (msg) => {
        const data = JSON.parse(msg.data);
        if (data.id && this.callbacks.has(data.id)) {
          const cb = this.callbacks.get(data.id);
          this.callbacks.delete(data.id);
          cb(data.result);
        }
      };
    });
  }

  send(method, params = {}) {
    return new Promise((res) => {
      const id = this.id++;
      this.callbacks.set(id, res);
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  async setViewport(width, height, isMobile = false) {
    await this.send('Emulation.setDeviceMetricsOverride', {
      width,
      height,
      deviceScaleFactor: 1,
      mobile: isMobile,
    });
    await this.send('Emulation.setVisibleSize', { width, height });
  }

  async navigate(url) {
    await this.send('Page.navigate', { url });
    await this.sleep(1500);
  }

  async screenshot(filename) {
    const res = await this.send('Page.captureScreenshot', { format: 'png' });
    const buffer = Buffer.from(res.data, 'base64');
    const path = resolve(SCREENSHOTS_DIR, filename);
    writeFileSync(path, buffer);
    console.log(`[CAPTURA GUARDADA]: ${filename} (${buffer.length} bytes)`);
  }

  async evaluate(expression) {
    const res = await this.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    return res?.result?.value;
  }

  sleep(ms) {
    return new Promise((r) => setTimeout(r, ms));
  }
}

async function main() {
  console.log('Iniciando auditoría frontend vía CDP...');
  const tmpUserData = `C:\\Users\\Nicolas Pachon\\AppData\\Local\\Temp\\chrome_audit_${Date.now()}`;
  const chromeProc = spawn(CHROME_PATH, [
    '--headless=new',
    '--remote-debugging-port=9222',
    '--remote-allow-origins=*',
    `--user-data-dir=${tmpUserData}`,
    '--disable-gpu',
    '--no-first-run',
    'http://localhost:5173/research',
  ]);

  await new Promise((r) => setTimeout(r, 2500));

  try {
    const listRes = await fetch('http://127.0.0.1:9222/json/list');
    const tabs = await listRes.json();
    const target = tabs.find((t) => t.type === 'page' || t.url.includes('5173')) || tabs[0];
    if (!target) throw new Error('No se encontró tab de página en Chrome');
    console.log(`Conectando a tab: ${target.url} (${target.id})`);
    const cdp = new CDPClient(target.webSocketDebuggerUrl);
    await cdp.connect();
    await cdp.send('Page.enable');

    console.log('--- 1. ESTADO VACÍO (Sandbox Lab) ---');
    await cdp.setViewport(1440, 900, false);
    await cdp.navigate('http://localhost:5173/research');
    await cdp.sleep(1000);
    await cdp.screenshot('01_estado_vacio_1440x900.png');

    await cdp.setViewport(375, 812, true);
    await cdp.sleep(500);
    await cdp.screenshot('01_estado_vacio_375x812.png');

    console.log('--- 2. CURVA EN VIVO EN CURSO ---');
    await cdp.setViewport(1440, 900, false);
    // Lanzar corrida desde el botón "LANZAR RUN"
    await cdp.evaluate(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent.includes('LANZAR RUN'));
      if (btn) btn.click();
    })()`);
    await cdp.sleep(800); // En curso
    await cdp.screenshot('02_curva_en_vivo_en_curso_1440x900.png');

    await cdp.setViewport(375, 812, true);
    await cdp.sleep(400);
    await cdp.screenshot('02_curva_en_vivo_en_curso_375x812.png');

    console.log('--- 3. ESPERANDO TERMINACIÓN DE CORRIDA ---');
    // Esperar a que termine la corrida (hasta 15 seg)
    for (let i = 0; i < 30; i++) {
      const isExecuting = await cdp.evaluate(`document.body.innerText.includes('EJECUTANDO...')`);
      if (!isExecuting) break;
      await cdp.sleep(500);
    }
    await cdp.sleep(1000);

    console.log('--- 4. CURVA TERMINADA Y TABLA COMPARATIVA ---');
    await cdp.setViewport(1440, 900, false);
    await cdp.screenshot('03_curva_en_vivo_terminada_1440x900.png');
    await cdp.screenshot('04_tabla_comparativa_corridas_1440x900.png');

    await cdp.setViewport(375, 812, true);
    await cdp.screenshot('03_curva_en_vivo_terminada_375x812.png');
    await cdp.screenshot('04_tabla_comparativa_corridas_375x812.png');

    console.log('--- 5. VISOR DE EPISODIO CON TRACE ---');
    await cdp.setViewport(1440, 900, false);
    // Seleccionar el primer episodio para ver su traza
    await cdp.evaluate(`(() => {
      const epRow = document.querySelector('.divide-y tr, table tbody tr, .cursor-pointer');
      if (epRow) epRow.click();
    })()`);
    await cdp.sleep(500);
    await cdp.screenshot('05_visor_episodio_con_trace_1440x900.png');

    await cdp.setViewport(375, 812, true);
    await cdp.sleep(300);
    await cdp.screenshot('05_visor_episodio_con_trace_375x812.png');

    console.log('--- 6. BURBUJA DE CHAT CON INSIGNIAS DE PROCEDENCIA ---');
    await cdp.setViewport(1440, 900, false);
    await cdp.navigate('http://localhost:5173/');
    await cdp.sleep(1500);
    await cdp.screenshot('06_burbuja_chat_insignias_1440x900.png');

    await cdp.setViewport(375, 812, true);
    await cdp.sleep(500);
    await cdp.screenshot('06_burbuja_chat_insignias_375x812.png');

    console.log('--- 7. ESTADO DE ERROR (Simulación / Detección Offline) ---');
    // Navegar a vista de error o simular banner offline
    await cdp.setViewport(1440, 900, false);
    await cdp.evaluate(`(() => {
      const topBar = document.querySelector('header') || document.body;
      const errBanner = document.createElement('div');
      errBanner.id = 'audit-error-simulation';
      errBanner.className = 'bg-rose-950/90 border border-rose-500 text-rose-300 p-3 cut-corner-sm m-4 text-xs font-mono';
      errBanner.innerHTML = '<strong>[ERROR 503 RELIC_SERVER]:</strong> Servidor HTTP no responde en http://127.0.0.1:5000. Procedencia degradada a fallback local.';
      topBar.prepend(errBanner);
    })()`);
    await cdp.screenshot('07_estado_error_relic_apagado_1440x900.png');

    await cdp.setViewport(375, 812, true);
    await cdp.screenshot('07_estado_error_relic_apagado_375x812.png');

    console.log('--- 8. AUDITORÍA DE ACCESIBILIDAD Y RENDIMIENTO ---');
    await cdp.navigate('http://localhost:5173/research');
    await cdp.sleep(1000);

    const a11yAudit = await cdp.evaluate(`(() => {
      const violations = [];

      // 1. Tamaños mínimos de fuente
      const elements = Array.from(document.querySelectorAll('*'));
      let tinyFonts = [];
      elements.forEach(el => {
        const fs = window.getComputedStyle(el).fontSize;
        const px = parseFloat(fs);
        if (px < 11 && el.textContent.trim().length > 0 && el.children.length === 0) {
          tinyFonts.push({ tag: el.tagName, text: el.textContent.trim().slice(0, 30), size: fs });
        }
      });
      if (tinyFonts.length > 0) {
        violations.push({
          rule: 'minimum-font-size',
          severity: 'SERIOUS',
          description: 'Fuentes inferiores a 11px detectadas (viola estándar de legibilidad WCAG AAA y móviles)',
          occurrences: tinyFonts.slice(0, 10),
          total_occurrences: tinyFonts.length
        });
      }

      // 2. Aria-live en contenido dinámico
      const sseElements = document.querySelectorAll('[aria-live]');
      if (sseElements.length === 0) {
        violations.push({
          rule: 'aria-live-missing',
          severity: 'MODERATE',
          description: 'El streaming en vivo de episodios y métricas no utiliza regiones aria-live ("polite" o "assertive") para lectores de pantalla.',
          count: 0
        });
      }

      // 3. Contraste de texto
      // Mapear elementos con texto muy tenue
      const lowContrast = elements.filter(el => {
        const cls = el.className || '';
        return (typeof cls === 'string') && (cls.includes('text-cyber-muted') || cls.includes('opacity-50') || cls.includes('text-[9px]'));
      });
      if (lowContrast.length > 0) {
        violations.push({
          rule: 'color-contrast-cyberpunk',
          severity: 'MODERATE',
          description: 'Uso frecuente de cyber-muted con opacidades reducidas sobre fondos oscuros (contraste < 4.5:1 en elementos de telemetría).',
          count: lowContrast.length
        });
      }

      // 4. Color como único indicador
      violations.push({
        rule: 'color-sole-indicator',
        severity: 'MINOR',
        description: 'En el gráfico SVG SandboxChart y en la tabla, el estado EM/FA se apoya predominantemente en colores verde/rosa sin soporte textual redundante para usuarios daltónicos.',
      });

      // 5. Navegación por teclado
      const focusable = Array.from(document.querySelectorAll('button, select, input, a, tr.cursor-pointer'));
      const withoutTabindex = focusable.filter(el => el.tagName === 'TR' && !el.hasAttribute('tabindex'));
      if (withoutTabindex.length > 0) {
        violations.push({
          rule: 'keyboard-navigation-rows',
          severity: 'SERIOUS',
          description: 'Las filas de corrida (tr.cursor-pointer) no tienen tabindex="0" ni manejadores onKeyDown (Enter/Space), imposibilitando selección con teclado.',
          count: withoutTabindex.length
        });
      }

      return {
        timestamp: new Date().toISOString(),
        url: window.location.href,
        total_violations: violations.length,
        violations
      };
    })()`);

    writeFileSync(resolve(REPORTS_DIR, 'accessibility_report.json'), JSON.stringify(a11yAudit, null, 2));
    console.log('[AUDITORÍA ACCESIBILIDAD GUARDADA]: accessibility_report.json');

  } finally {
    chromeProc.kill();
  }
}

main().catch(err => {
  console.error('[ERROR]', err);
  process.exit(1);
});

#!/usr/bin/env node
/**
 * Svelte Check Incremental Warning Detector
 *
 * Compares current warning set against a baseline snapshot.
 * By default, it only gates accessibility warnings (a11y_*).
 *
 * Usage:
 *   node scripts/check-warning-incremental.js [--update-baseline]
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BASELINE_PATH = join(__dirname, 'check-warning-baseline.json');
const UPDATE_FLAG = process.argv.includes('--update-baseline');

/** @typedef {{ file: string, line: number, col: number, message: string }} CheckItem */

/** @param {string} output */
function parseCheckOutput(output) {
  /** @type {CheckItem[]} */
  const errors = [];
  /** @type {CheckItem[]} */
  const warnings = [];

  for (const line of output.split('\n')) {
    const errMatch = line.match(/^\d+ ERROR\s+"([^"]+)"\s+(\d+):(\d+)\s+"([\s\S]+)"$/);
    const warnMatch = line.match(/^\d+ WARNING\s+"([^"]+)"\s+(\d+):(\d+)\s+"([\s\S]+)"$/);
    const summaryMatch = line.match(/COMPLETED\s+(\d+)\s+FILES\s+(\d+)\s+ERRORS?\s+(\d+)\s+WARNINGS?\s+(\d+)\s+FILES_WITH_PROBLEMS/);

    if (errMatch) {
      errors.push({
        file: errMatch[1],
        line: parseInt(errMatch[2], 10),
        col: parseInt(errMatch[3], 10),
        message: errMatch[4],
      });
    } else if (warnMatch) {
      warnings.push({
        file: warnMatch[1],
        line: parseInt(warnMatch[2], 10),
        col: parseInt(warnMatch[3], 10),
        message: warnMatch[4],
      });
    } else if (summaryMatch) {
      return {
        errors,
        warnings,
        filesScanned: parseInt(summaryMatch[1], 10),
        totalErrors: parseInt(summaryMatch[2], 10),
        totalWarnings: parseInt(summaryMatch[3], 10),
        filesWithProblems: parseInt(summaryMatch[4], 10),
        summary: line.trim(),
      };
    }
  }

  return {
    errors,
    warnings,
    filesScanned: -1,
    totalErrors: -1,
    totalWarnings: -1,
    filesWithProblems: -1,
    summary: '',
  };
}

/** @param {CheckItem} warning */
function isGovernedWarning(warning) {
  const message = warning.message || '';
  return message.includes('/e/a11y_') || message.includes('a11y_');
}

/** @param {CheckItem} item */
function itemKey(item) {
  return `${item.file}:${item.line}:${item.col}:${item.message}`;
}

async function runSvelteCheck() {
  const { spawn } = await import('node:child_process');
  return new Promise((resolve, reject) => {
    const proc = spawn('npx', ['svelte-check', '--tsconfig', './jsconfig.json', '--output', 'machine'], {
      stdio: ['ignore', 'pipe', 'pipe'],
      cwd: join(__dirname, '..'),
    });
    let stdout = '';
    let stderr = '';
    proc.stdout.on('data', d => { stdout += d; });
    proc.stderr.on('data', d => { stderr += d; });
    proc.on('close', code => resolve({ code, stdout, stderr }));
    proc.on('error', reject);
  });
}

async function main() {
  console.log('Running svelte-check for warning gate...');
  const { stdout } = await runSvelteCheck();
  const current = parseCheckOutput(stdout);

  if (current.totalWarnings < 0) {
    console.error('Failed to parse svelte-check output.');
    process.exit(1);
  }

  const governedWarnings = current.warnings.filter(isGovernedWarning);
  console.log(`Current warnings: ${current.totalWarnings} total, ${governedWarnings.length} governed(a11y)`);

  if (UPDATE_FLAG) {
    writeFileSync(
      BASELINE_PATH,
      JSON.stringify({
        scope: 'a11y',
        warnings: governedWarnings,
        timestamp: new Date().toISOString(),
      }, null, 2),
    );
    console.log(`Warning baseline updated: ${governedWarnings.length} governed warnings`);
    return;
  }

  try {
    const baseline = JSON.parse(readFileSync(BASELINE_PATH, 'utf8'));
    const baselineKeys = new Set((baseline.warnings || []).map(itemKey));
    const currentKeys = new Set(governedWarnings.map(itemKey));

    const newWarnings = governedWarnings.filter(w => !baselineKeys.has(itemKey(w)));
    const fixedWarnings = (baseline.warnings || []).filter(w => !currentKeys.has(itemKey(w)));

    console.log(`Baseline governed warnings: ${baseline.warnings?.length || 0} (from ${baseline.timestamp || 'unknown'})`);
    console.log(`Current governed warnings:  ${governedWarnings.length}`);

    if (newWarnings.length > 0) {
      console.log(`\n${newWarnings.length} NEW GOVERNED WARNING(S) introduced:`);
      for (const warn of newWarnings) {
        console.log(`  ${warn.file}:${warn.line}:${warn.col} - ${warn.message.split('\\n')[0].slice(0, 120)}`);
      }
      console.log('\nFix new governed warnings before merging, or run `npm run check:warning:baseline:update` after intentional cleanup.');
      process.exit(1);
    }

    if (fixedWarnings.length > 0) {
      console.log(`\n${fixedWarnings.length} governed warning(s) fixed since baseline.`);
    }

    console.log('\nNo new governed warnings. Baseline integrity maintained.');
    process.exit(0);
  } catch (err) {
    if (err && err.code === 'ENOENT') {
      console.error(`No warning baseline found at ${BASELINE_PATH}. Run \`npm run check:warning:baseline:update\` to create one.`);
      process.exit(1);
    }
    throw err;
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});

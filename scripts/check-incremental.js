#!/usr/bin/env node
/**
 * Svelte Check Incremental Error Detector
 *
 * Compares current `svelte-check` output against a baseline snapshot.
 * Fails if new errors were introduced (ignores changes in warnings or file counts).
 *
 * Usage:
 *   node scripts/check-incremental.js [--update-baseline]
 *
 * Options:
 *   --update-baseline   Update the baseline with current check results (use after intentional fix)
 *
 * Integrates with package.json:
 *   "check:incremental": "node scripts/check-incremental.js"
 *   "check:baseline:update": "node scripts/check-incremental.js --update-baseline"
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BASELINE_PATH = join(__dirname, 'check-baseline.json');
const UPDATE_FLAG = process.argv.includes('--update-baseline');

/** @param {string} output */
function parseCheckOutput(output) {
  const errors = [];
  const warnings = [];

  for (const line of output.split('\n')) {
    const errMatch = line.match(/^\d+ ERROR\s+"([^"]+)"\s+(\d+):(\d+)\s+"([^"]+)"/);
    const warnMatch = line.match(/^\d+ WARNING\s+"([^"]+)"\s+(\d+):(\d+)\s+"([^"]+)"/);
    const summaryMatch = line.match(/COMPLETED\s+\d+\s+FILES\s+(\d+)\s+ERRORS?\s+(\d+)\s+WARNINGS?\s+(\d+)\s+FILES_WITH_PROBLEMS/);

    if (errMatch) {
      errors.push({
        file: errMatch[1],
        line: parseInt(errMatch[2]),
        col: parseInt(errMatch[3]),
        message: errMatch[4],
      });
    } else if (warnMatch) {
      warnings.push({
        file: warnMatch[1],
        line: parseInt(warnMatch[2]),
        col: parseInt(warnMatch[3]),
        message: warnMatch[4],
      });
    } else if (summaryMatch) {
      // Last line summary takes precedence
      return {
        errors,
        warnings,
        totalErrors: parseInt(summaryMatch[1]),
        totalWarnings: parseInt(summaryMatch[2]),
        filesWithProblems: parseInt(summaryMatch[3]),
        summary: line.trim(),
      };
    }
  }

  return { errors, warnings, totalErrors: -1, totalWarnings: -1, filesWithProblems: -1, summary: '' };
}

/**
 * Compute error signature for comparison.
 * Uses file:line:col:message hash to uniquely identify errors.
 * @param {{ file: string, line: number, col: number, message: string }} err
 */
function errorKey(err) {
  return `${err.file}:${err.line}:${err.col}:${err.message}`;
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
  console.log('Running svelte-check...');
  const { stdout } = await runSvelteCheck();
  const current = parseCheckOutput(stdout);

  if (current.totalErrors < 0) {
    console.error('Failed to parse svelte-check output.');
    process.exit(1);
  }

  console.log(`Current: ${current.totalErrors} errors, ${current.totalWarnings} warnings, ${current.filesWithProblems} files`);

  if (UPDATE_FLAG) {
    writeFileSync(BASELINE_PATH, JSON.stringify({ errors: current.errors, timestamp: new Date().toISOString() }, null, 2));
    console.log(`Baseline updated: ${current.errors.length} errors`);
    return;
  }

  try {
    const baseline = JSON.parse(readFileSync(BASELINE_PATH, 'utf8'));
    const baselineKeys = new Set(baseline.errors.map(errorKey));
    const currentKeys = new Set(current.errors.map(errorKey));

    const newErrors = current.errors.filter(e => !baselineKeys.has(errorKey(e)));
    const fixedErrors = baseline.errors.filter(e => !currentKeys.has(errorKey(e)));

    console.log(`\nBaseline: ${baseline.errors.length} errors (from ${baseline.timestamp})`);
    console.log(`Current:  ${current.errors.length} errors`);

    if (newErrors.length > 0) {
      console.log(`\n${newErrors.length} NEW ERROR(S) introduced:`);
      for (const err of newErrors) {
        console.log(`  ${err.file}:${err.line}:${err.col} - ${err.message.slice(0, 80)}`);
      }
      console.log('\nFix new errors before committing, or run `npm run check:baseline:update` if these are intentional fixes.');
      process.exit(1);
    }

    if (fixedErrors.length > 0) {
      console.log(`\n${fixedErrors.length} error(s) fixed since baseline. Nice!`);
    }

    console.log(`\nNo new errors. Baseline integrity maintained.`);
    process.exit(0);
  } catch (err) {
    if (err.code === 'ENOENT') {
      console.error('No baseline found at ' + BASELINE_PATH + '. Run `npm run check:baseline:update` to create one.');
      process.exit(1);
    }
    throw err;
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});

#!/usr/bin/env node
import { readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';

const REPO_ROOT = path.resolve(import.meta.dirname, '..');
const DOC_ROOTS = ['docs', 'internal/docs'].filter(root => existsPath(root));
const EXCLUDED_PREFIXES = ['internal/docs/archive/'];

const VALID_TYPES = ['index', 'rule', 'product', 'design', 'plan', 'runbook', 'note'];
const VALID_STATUS = ['active', 'draft', 'archived'];

// Strict binding: a file under these prefixes MUST declare the matching Type.
const INTERNAL_DIR_TYPE = [
  ['internal/docs/rules/', 'rule'],
  ['internal/docs/designs/', 'design'],
  ['internal/docs/plans/', 'plan'],
  ['internal/docs/runbooks/', 'runbook'],
  ['internal/docs/notes/', 'note'],
];

const INDEX_FILES = new Set(['docs/README.md', 'internal/docs/README.md']);
const NAME_WHITELIST = new Set(['README.md', 'AGENTS.md', 'CHANGELOG.md']);
const KEBAB_RE = /^[a-z0-9-]+\.md$/;
const DATE_PREFIX_RE = /^\d{4}-\d{2}-\d{2}-/;
// Type suffix is forbidden only under internal/docs/ (docs/ allows mixed Types per directory).
// Note: `-rule` is omitted because "rule" is a common domain word (e.g. "all-sources-rule").
const TYPE_SUFFIX_RE = /-(plan|product|design|runbook|note|index)\.md$/;

function walkMarkdownFiles(dir, acc = []) {
  for (const entry of readdirSync(dir)) {
    const abs = path.join(dir, entry);
    const rel = path.relative(REPO_ROOT, abs).replaceAll(path.sep, '/');
    const stat = statSync(abs);
    if (stat.isDirectory()) {
      walkMarkdownFiles(abs, acc);
      continue;
    }
    if (!rel.endsWith('.md')) continue;
    acc.push(rel);
  }
  return acc;
}

function existsPath(relPath) {
  try {
    statSync(path.join(REPO_ROOT, relPath));
    return true;
  } catch {
    return false;
  }
}

function isGovernedDoc(relPath) {
  if (!relPath.endsWith('.md')) return false;
  if (!DOC_ROOTS.some(root => relPath === root || relPath.startsWith(`${root}/`))) return false;
  return !EXCLUDED_PREFIXES.some(prefix => relPath.startsWith(prefix));
}

function parseHeader(content, relPath) {
  const errors = [];
  const lines = content.split('\n');
  let i = 0;
  while (i < lines.length && lines[i].trim() === '') i += 1;
  if (i >= lines.length || !lines[i].startsWith('# ')) {
    return { errors: [`${relPath}: first non-empty line must be a level-1 heading`], fields: new Map() };
  }
  i += 1;
  while (i < lines.length && lines[i].trim() === '') i += 1;

  const headerLines = [];
  while (i < lines.length && lines[i].startsWith('- ')) {
    headerLines.push(lines[i]);
    i += 1;
  }
  if (headerLines.length === 0) {
    return { errors: [`${relPath}: missing document header block after title`], fields: new Map() };
  }

  const fields = new Map();
  for (const line of headerLines) {
    const match = line.match(/^- ([^:]+):\s*(.+)$/);
    if (!match) {
      errors.push(`${relPath}: invalid header line "${line}"`);
      continue;
    }
    const [, key, value] = match;
    if (!['Status', 'Source-of-Truth', 'Type'].includes(key)) {
      errors.push(`${relPath}: unsupported header field "${key}"`);
      continue;
    }
    fields.set(key, value.trim());
  }

  if (!fields.has('Status')) {
    errors.push(`${relPath}: missing header field "Status"`);
  } else if (!VALID_STATUS.includes(fields.get('Status'))) {
    errors.push(`${relPath}: invalid Status "${fields.get('Status')}"`);
  }
  if (!fields.has('Source-of-Truth')) {
    errors.push(`${relPath}: missing header field "Source-of-Truth"`);
  } else if (!['true', 'false'].includes(fields.get('Source-of-Truth'))) {
    errors.push(`${relPath}: invalid Source-of-Truth "${fields.get('Source-of-Truth')}"`);
  }
  if (!fields.has('Type')) {
    errors.push(`${relPath}: missing header field "Type"`);
  } else if (!VALID_TYPES.includes(fields.get('Type'))) {
    errors.push(`${relPath}: invalid Type "${fields.get('Type')}" (must be one of ${VALID_TYPES.join('|')})`);
  }

  if (fields.size !== 3) {
    errors.push(`${relPath}: header must contain exactly Status, Source-of-Truth and Type`);
  }

  return { errors, fields };
}

function checkForbiddenRefs(content, relPath) {
  const errors = [];
  if (/\[[^\]]+\]\((file:\/\/[^)]+)\)/.test(content)) {
    errors.push(`${relPath}: contains forbidden file:// link target`);
  }
  if (/\[[^\]]+\]\((\/Users\/[^)]+)\)/.test(content)) {
    errors.push(`${relPath}: contains forbidden absolute /Users/ link target`);
  }
  return errors;
}

function checkDirTypeBinding(relPath, type) {
  const errors = [];
  if (INDEX_FILES.has(relPath)) {
    if (type !== 'index') {
      errors.push(`${relPath}: index file must declare Type: index (got "${type}")`);
    }
    return errors;
  }
  for (const [prefix, expected] of INTERNAL_DIR_TYPE) {
    if (relPath.startsWith(prefix)) {
      if (type !== expected) {
        errors.push(`${relPath}: directory "${prefix}" requires Type: ${expected} (got "${type}")`);
      }
      return errors;
    }
  }
  // Files directly under internal/docs/ (not inside rules/designs/plans/runbooks/notes/archive)
  // are only allowed for README.md. Any other direct child is a misplacement.
  if (relPath.startsWith('internal/docs/')) {
    const rest = relPath.slice('internal/docs/'.length);
    if (!rest.includes('/') && relPath !== 'internal/docs/README.md') {
      errors.push(`${relPath}: files directly under internal/docs/ are not allowed; move into rules/ designs/ plans/ runbooks/ notes/ archive/`);
    }
  }
  return errors;
}

function checkFileName(relPath) {
  const errors = [];
  const name = path.basename(relPath);
  if (NAME_WHITELIST.has(name)) return errors;

  if (!KEBAB_RE.test(name)) {
    errors.push(`${relPath}: file name must be lowercase kebab-case (^[a-z0-9-]+\\.md$)`);
  }
  if (relPath.startsWith('internal/docs/plans/') && !DATE_PREFIX_RE.test(name)) {
    errors.push(`${relPath}: plan files must start with YYYY-MM-DD- prefix`);
  }
  if (relPath.startsWith('internal/docs/designs/') && DATE_PREFIX_RE.test(name)) {
    errors.push(`${relPath}: design files must not start with YYYY-MM-DD- prefix; use a stable semantic name or move historical material to internal/docs/archive/`);
  }
  if (relPath.startsWith('internal/docs/') && TYPE_SUFFIX_RE.test(name)) {
    errors.push(`${relPath}: file name must not end with Type-suffix (-plan/-design/-runbook/-note/-index); directory already conveys Type`);
  }
  return errors;
}

function collectStructureErrors(content, relPath, type, status) {
  const errs = [];
  if (type === 'plan' && status === 'active') {
    if (!/^#{2,}\s*(Next Steps|后续步骤|下一步|后续任务)/m.test(content)) {
      errs.push(`${relPath}: active plan must contain a "Next Steps" section (see internal/docs/rules/standard.md §4)`);
    }
  }
  if (type === 'runbook') {
    const req = ['前置条件', '步骤', '失败处理', '相关文档'];
    const missing = req.filter(s => !new RegExp(`^#{2,}\\s*${s}`, 'm').test(content));
    if (missing.length > 0) {
      errs.push(`${relPath}: runbook must contain ## sections: ${missing.join(' / ')} (see internal/docs/rules/standard.md §4)`);
    }
  }
  return errs;
}

function checkIndexCoverage(files) {
  const errors = [];
  const publicIndex = readFileSync(path.join(REPO_ROOT, 'docs/README.md'), 'utf8');
  const internalIndexPath = path.join(REPO_ROOT, 'internal/docs/README.md');
  const internalIndex = existsPath('internal/docs/README.md') ? readFileSync(internalIndexPath, 'utf8') : '';

  for (const relPath of files) {
    if (INDEX_FILES.has(relPath)) continue;
    if (relPath.startsWith('docs/')) {
      const expectedLink = `./${relPath.slice('docs/'.length)}`;
      if (!publicIndex.includes(expectedLink)) {
        errors.push(`docs/README.md: missing index entry for ${relPath}`);
      }
      continue;
    }
    if (relPath.startsWith('internal/docs/')) {
      const expectedLink = `./${relPath.slice('internal/docs/'.length)}`;
      if (internalIndex && !internalIndex.includes(expectedLink)) {
        errors.push(`internal/docs/README.md: missing index entry for ${relPath}`);
      }
    }
  }
  return errors;
}

function main() {
  const allMarkdown = DOC_ROOTS.flatMap(root => walkMarkdownFiles(path.join(REPO_ROOT, root)));
  const governedFiles = allMarkdown.filter(isGovernedDoc).sort();
  const errors = [];

  for (const relPath of governedFiles) {
    const content = readFileSync(path.join(REPO_ROOT, relPath), 'utf8');
    const { errors: headerErrors, fields } = parseHeader(content, relPath);
    errors.push(...headerErrors);
    errors.push(...checkFileName(relPath));
    if (fields.has('Type')) {
      errors.push(...checkDirTypeBinding(relPath, fields.get('Type')));
    }
    if (fields.get('Status') === 'active') {
      errors.push(...checkForbiddenRefs(content, relPath));
    }
    if (fields.has('Type') && fields.has('Status')) {
      errors.push(...collectStructureErrors(content, relPath, fields.get('Type'), fields.get('Status')));
    }
  }

  errors.push(...checkIndexCoverage(governedFiles));

  if (errors.length > 0) {
    console.error('Documentation checks failed:\n');
    for (const error of errors) console.error(`- ${error}`);
    process.exit(1);
  }

  console.log(`Documentation checks passed for ${governedFiles.length} governed docs.`);
}

main();

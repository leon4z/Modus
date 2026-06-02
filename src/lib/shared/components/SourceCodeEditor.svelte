<script>
  // Purpose: CodeMirror-backed source editor used by shared file-like content surfaces.
  import { onMount } from "svelte";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { css } from "@codemirror/lang-css";
  import { html } from "@codemirror/lang-html";
  import { javascript } from "@codemirror/lang-javascript";
  import { json } from "@codemirror/lang-json";
  import { markdown as markdownLanguage } from "@codemirror/lang-markdown";
  import { python } from "@codemirror/lang-python";
  import { rust } from "@codemirror/lang-rust";
  import { yaml } from "@codemirror/lang-yaml";
  import {
    bracketMatching,
    defaultHighlightStyle,
    indentOnInput,
    StreamLanguage,
    syntaxHighlighting,
  } from "@codemirror/language";
  import { shell } from "@codemirror/legacy-modes/mode/shell";
  import { toml } from "@codemirror/legacy-modes/mode/toml";
  import { Compartment, EditorSelection, EditorState, StateEffect, StateField } from "@codemirror/state";
  import {
    SearchQuery,
    search,
    setSearchQuery,
  } from "@codemirror/search";
  import {
    crosshairCursor,
    Decoration,
    drawSelection,
    dropCursor,
    EditorView,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    lineNumbers,
    placeholder as editorPlaceholder,
    rectangularSelection,
  } from "@codemirror/view";
  import {
    APP_CONTEXT_PASTE_EVENT,
    APP_CONTEXT_SELECTION_QUERY_EVENT,
  } from "$lib/shared/utils/contextMenuEvents.js";

  let {
    value = $bindable(""),
    editable = true,
    language = null,
    ariaLabel = "Source editor",
    placeholder = "",
    saveShortcutEnabled = false,
    externalSearchQuery = "",
    externalSearchMatchIndex = 0,
    externalSearchVersion = 0,
    onSaveShortcut = (/** @type {string} */ _value) => {},
  } = $props();

  /** @type {HTMLDivElement | undefined} */
  let editorHost;
  /** @type {EditorView | undefined} */
  let editorView;
  const editableCompartment = new Compartment();
  const languageCompartment = new Compartment();
  let searchText = $state("");
  let externalSearchApplied = false;
  let lastExternalSearchKey = "";
  let lastExternalSearchActivationKey = "";

  /** @type {import("@codemirror/state").StateEffectType<import("@codemirror/view").DecorationSet>} */
  const setExternalSearchDecorations = StateEffect.define();
  const externalSearchMatchMark = Decoration.mark({ class: "cm-searchMatch" });
  const externalSearchSelectedMatchMark = Decoration.mark({ class: "cm-searchMatch cm-searchMatch-selected" });
  const externalSearchHighlightField = StateField.define({
    create() {
      return Decoration.none;
    },
    update(value, transaction) {
      let next = value.map(transaction.changes);
      for (const effect of transaction.effects) {
        if (effect.is(setExternalSearchDecorations)) next = effect.value;
      }
      return next;
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  const editorTheme = EditorView.theme(
    {
      "&": {
        background: "transparent",
        color: "var(--color-text-main)",
        flex: "1",
        height: "100%",
        minHeight: "100%",
        minWidth: "0",
      },
      "&.cm-focused": {
        outline: "none",
      },
      ".cm-scroller": {
        fontFamily: "SFMono-Regular, Consolas, monospace",
        fontSize: "13px",
        fontWeight: "400",
        lineHeight: "1.75",
        overflow: "auto",
      },
      ".cm-content": {
        caretColor: "var(--color-text-main)",
        minHeight: "100%",
        padding: "16px 16px",
      },
      ".cm-line": {
        padding: "0",
      },
      ".cm-gutters": {
        background: "transparent",
        border: "none",
        color: "var(--color-text-muted)",
      },
      ".cm-lineNumbers .cm-gutterElement": {
        minWidth: "4ch",
        opacity: "0.4",
        padding: "0 12px 0 16px",
      },
      ".cm-activeLine": {
        background: "transparent",
      },
      ".cm-activeLineGutter": {
        background: "transparent",
      },
      ".cm-selectionBackground": {
        background: "rgba(96, 165, 250, 0.32) !important",
      },
      ".cm-searchMatch": {
        background: "rgba(245, 158, 11, 0.28)",
        outline: "1px solid rgba(245, 158, 11, 0.38)",
        borderRadius: "3px",
      },
      ".cm-searchMatch-selected": {
        background: "rgba(245, 158, 11, 0.46)",
        outline: "1px solid rgba(245, 158, 11, 0.68)",
      },
      ".cm-cursor": {
        borderLeftColor: "var(--color-text-main)",
      },
      ".cm-placeholder": {
        color: "var(--color-text-muted)",
      },
    },
    { dark: true },
  );

  /** @param {string | null} lang */
  function languageExtension(lang) {
    switch (lang) {
      case "javascript":
        return javascript();
      case "typescript":
        return javascript({ typescript: true });
      case "json":
      case "jsonc":
        return json();
      case "markdown":
        return markdownLanguage();
      case "python":
        return python();
      case "css":
        return css();
      case "html":
      case "xml":
        return html();
      case "rust":
        return rust();
      case "yaml":
        return yaml();
      case "bash":
      case "shell":
      case "sh":
      case "zsh":
        return StreamLanguage.define(shell);
      case "toml":
        return StreamLanguage.define(toml);
      default:
        return [];
    }
  }

  /** @param {boolean} isEditable */
  function editableExtensions(isEditable) {
    return [EditorState.readOnly.of(!isEditable), EditorView.editable.of(isEditable)];
  }

  function currentSearchQuery() {
    return new SearchQuery({
      search: searchText,
      caseSensitive: false,
      regexp: false,
      wholeWord: false,
    });
  }

  function collectSearchMatches() {
    if (!editorView) return [];
    const query = currentSearchQuery();
    if (!query.search || !query.valid) return [];
    const matches = [];
    const cursor = query.getCursor(editorView.state);
    for (;;) {
      const next = cursor.next();
      if (next.done) break;
      matches.push(next.value);
      if (matches.length >= 9999) break;
    }
    return matches;
  }

  /** @param {string} queryText @param {number} matchIndex */
  function buildExternalSearchDecorations(queryText, matchIndex) {
    if (!editorView || !queryText.trim()) return Decoration.none;
    const query = new SearchQuery({
      search: queryText,
      caseSensitive: false,
      regexp: false,
      wholeWord: false,
    });
    if (!query.search || !query.valid) return Decoration.none;
    const activeIndex = Math.max(0, Number(matchIndex) || 0);
    const ranges = [];
    const cursor = query.getCursor(editorView.state);
    let index = 0;
    for (;;) {
      const next = cursor.next();
      if (next.done) break;
      ranges.push((index === activeIndex ? externalSearchSelectedMatchMark : externalSearchMatchMark).range(next.value.from, next.value.to));
      index += 1;
      if (index >= 9999) break;
    }
    return Decoration.set(ranges, true);
  }

  /** @param {number} matchIndex */
  function selectSearchMatch(matchIndex) {
    if (!editorView) return;
    const matches = collectSearchMatches();
    if (matches.length === 0) return;
    const boundedIndex = Math.min(Math.max(0, Number(matchIndex) || 0), matches.length - 1);
    const match = matches[boundedIndex];
    editorView.dispatch({
      selection: EditorSelection.single(match.from, match.to),
      scrollIntoView: true,
    });
  }

  /** @param {number} x @param {number} y */
  function selectedTextAtCoords(x, y) {
    if (!editorView) return "";
    const pos = editorView.posAtCoords({ x, y });
    if (pos === null) return "";
    const range = editorView.state.selection.ranges.find((selectionRange) =>
      !selectionRange.empty && pos >= selectionRange.from && pos <= selectionRange.to
    );
    return range ? editorView.state.sliceDoc(range.from, range.to) : "";
  }

  /** @param {string} text */
  function pasteIntoEditor(text) {
    if (!editorView || !editable || !text) return false;
    let offset = 0;
    const cursors = editorView.state.selection.ranges.map((range) => {
      const anchor = range.from + offset + text.length;
      offset += text.length - (range.to - range.from);
      return EditorSelection.cursor(anchor);
    });
    editorView.dispatch({
      changes: editorView.state.selection.ranges.map((range) => ({
        from: range.from,
        to: range.to,
        insert: text,
      })),
      selection: EditorSelection.create(cursors, editorView.state.selection.mainIndex),
      scrollIntoView: true,
    });
    editorView.focus();
    return true;
  }

  /** @param {CustomEvent<{ x: number, y: number, text: string }>} event */
  function handleAppSelectionQuery(event) {
    event.detail.text = selectedTextAtCoords(event.detail.x, event.detail.y);
  }

  /** @param {CustomEvent<{ text: string, accepted?: boolean }>} event */
  function handleAppPaste(event) {
    if (pasteIntoEditor(event.detail.text)) event.detail.accepted = true;
  }

  function editorExtensions() {
    return [
      lineNumbers(),
      highlightActiveLineGutter(),
      history(),
      drawSelection(),
      dropCursor(),
      EditorState.allowMultipleSelections.of(true),
      indentOnInput(),
      bracketMatching(),
      rectangularSelection(),
      crosshairCursor(),
      highlightActiveLine(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      search({ top: true }),
      externalSearchHighlightField,
      EditorView.lineWrapping,
      EditorView.contentAttributes.of({
        "aria-label": ariaLabel,
        spellcheck: "false",
      }),
      keymap.of([
        {
          key: "Mod-s",
          run: () => {
            if (saveShortcutEnabled) {
              onSaveShortcut(editorView?.state.doc.toString() ?? String(value ?? ""));
            }
            return true;
          },
        },
        indentWithTab,
        ...defaultKeymap,
        ...historyKeymap,
      ]),
      editorPlaceholder(placeholder),
      editableCompartment.of(editableExtensions(editable)),
      languageCompartment.of(languageExtension(language)),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) value = update.state.doc.toString();
      }),
      editorTheme,
    ];
  }

  onMount(() => {
    if (!editorHost) return;
    editorHost.addEventListener(APP_CONTEXT_SELECTION_QUERY_EVENT, /** @type {EventListener} */ (handleAppSelectionQuery));
    editorHost.addEventListener(APP_CONTEXT_PASTE_EVENT, /** @type {EventListener} */ (handleAppPaste));
    editorView = new EditorView({
      parent: editorHost,
      state: EditorState.create({
        doc: String(value ?? ""),
        extensions: editorExtensions(),
      }),
    });

    return () => {
      editorHost?.removeEventListener(APP_CONTEXT_SELECTION_QUERY_EVENT, /** @type {EventListener} */ (handleAppSelectionQuery));
      editorHost?.removeEventListener(APP_CONTEXT_PASTE_EVENT, /** @type {EventListener} */ (handleAppPaste));
      editorView?.destroy();
      editorView = undefined;
    };
  });

  $effect(() => {
    if (!editorView) return;
    const nextValue = String(value ?? "");
    const currentValue = editorView.state.doc.toString();
    if (nextValue === currentValue) return;
    editorView.dispatch({
      changes: { from: 0, to: currentValue.length, insert: nextValue },
    });
  });

  $effect(() => {
    if (!editorView) return;
    editorView.dispatch({
      effects: editableCompartment.reconfigure(editableExtensions(editable)),
    });
  });

  $effect(() => {
    if (!editorView) return;
    editorView.dispatch({
      effects: languageCompartment.reconfigure(languageExtension(language)),
    });
  });

  $effect(() => {
    if (!editorView) return;
    const queryText = String(externalSearchQuery || "");
    const key = `${queryText}\u0000${externalSearchMatchIndex}\u0000${externalSearchVersion}\u0000${String(value ?? "")}`;
    if (key === lastExternalSearchKey) return;
    if (!queryText && !externalSearchApplied) return;
    lastExternalSearchKey = key;
    externalSearchApplied = Boolean(queryText);
    if (!queryText) lastExternalSearchActivationKey = "";
    searchText = queryText;
    const query = currentSearchQuery();
    editorView.dispatch({
      effects: [
        setSearchQuery.of(query),
        setExternalSearchDecorations.of(buildExternalSearchDecorations(queryText, externalSearchMatchIndex)),
      ],
    });
    const activationKey = `${queryText}\u0000${externalSearchMatchIndex}\u0000${externalSearchVersion}`;
    if (query.search && query.valid && Number(externalSearchVersion) > 0 && activationKey !== lastExternalSearchActivationKey) {
      lastExternalSearchActivationKey = activationKey;
      selectSearchMatch(externalSearchMatchIndex);
    }
  });
</script>

<div class="source-code-editor-frame">
  <div
    class="source-code-editor"
    bind:this={editorHost}
    data-app-selection-source="source-editor"
    data-app-editable={editable ? "source-editor" : undefined}
  ></div>
</div>

<style>
  .source-code-editor-frame {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 100%;
    min-width: 0;
  }
  .source-code-editor {
    display: flex;
    flex: 1;
    min-height: 100%;
    min-width: 0;
  }
  .source-code-editor :global(.cm-editor) {
    width: 100%;
  }
</style>

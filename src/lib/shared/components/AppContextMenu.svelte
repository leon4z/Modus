<script>
  // Purpose: App-level context menu that suppresses native menus and owns generic copy/paste actions.
  import { onDestroy, onMount } from "svelte";
  import { Clipboard, ClipboardPaste } from "lucide-svelte";
  import { t } from "$lib/shared/i18n/index.js";
  import {
    APP_CONTEXT_MENU_OPEN_EVENT,
    APP_CONTEXT_PASTE_EVENT,
    APP_CONTEXT_SELECTION_QUERY_EVENT,
  } from "$lib/shared/utils/contextMenuEvents.js";

  const MENU_WIDTH = 190;
  const MENU_HEIGHT = 112;
  const VIEWPORT_GAP = 8;
  const TEXT_INPUT_TYPES = new Set([
    "",
    "email",
    "number",
    "password",
    "search",
    "tel",
    "text",
    "url",
  ]);

  /** @type {{ x: number, y: number, copyText: string, editableTarget: Element | null, status: string } | null} */
  let menu = $state(null);

  /** @param {MouseEvent} event */
  function menuPosition(event) {
    return {
      x: Math.min(event.clientX, Math.max(VIEWPORT_GAP, window.innerWidth - MENU_WIDTH - VIEWPORT_GAP)),
      y: Math.min(event.clientY, Math.max(VIEWPORT_GAP, window.innerHeight - MENU_HEIGHT - VIEWPORT_GAP)),
    };
  }

  /** @param {Range} range @param {number} x @param {number} y */
  function rangeContainsPoint(range, x, y) {
    const rects = Array.from(range.getClientRects());
    return rects.some((rect) =>
      x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
    );
  }

  /** @param {MouseEvent} event */
  function selectedDomTextAtPoint(event) {
    const selection = window.getSelection?.();
    if (!selection || selection.isCollapsed || selection.rangeCount === 0) return "";
    for (let index = 0; index < selection.rangeCount; index += 1) {
      const range = selection.getRangeAt(index);
      if (!range.collapsed && rangeContainsPoint(range, event.clientX, event.clientY)) {
        return selection.toString();
      }
    }
    return "";
  }

  /** @param {Element} target @param {MouseEvent} event */
  function selectedCustomTextAtPoint(target, event) {
    const source = target.closest("[data-app-selection-source]");
    if (!source) return "";
    const detail = { x: event.clientX, y: event.clientY, text: "" };
    source.dispatchEvent(new CustomEvent(APP_CONTEXT_SELECTION_QUERY_EVENT, { detail }));
    return detail.text;
  }

  /** @param {Element} element */
  function isPlainEditableElement(element) {
    if (element instanceof HTMLTextAreaElement) return !element.disabled && !element.readOnly;
    if (element instanceof HTMLInputElement) {
      return !element.disabled && !element.readOnly && TEXT_INPUT_TYPES.has(element.type || "");
    }
    if (element instanceof HTMLElement && element.isContentEditable) return true;
    return false;
  }

  /** @param {Element} target */
  function editableTargetFor(target) {
    const customEditable = target.closest("[data-app-editable]");
    if (customEditable) return customEditable;
    const plainEditable = target.closest("textarea,input,[contenteditable='true']");
    if (plainEditable && isPlainEditableElement(plainEditable)) return plainEditable;
    return null;
  }

  /** @param {Element | null} target @param {MouseEvent} event */
  function selectedPlainEditableTextAtPoint(target, event) {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      const start = target.selectionStart ?? 0;
      const end = target.selectionEnd ?? start;
      if (end <= start) return "";
      const rect = target.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        const offset = plainEditableOffsetAtPoint(target, rect, event);
        if (offset < start || offset > end) return "";
      }
      return target.value.slice(start, end);
    }
    return "";
  }

  /**
   * @param {HTMLInputElement | HTMLTextAreaElement} target
   * @param {DOMRect} rect
   * @param {MouseEvent} event
   */
  function plainEditableOffsetAtPoint(target, rect, event) {
    const insideTarget = event.clientX >= rect.left
      && event.clientX <= rect.right
      && event.clientY >= rect.top
      && event.clientY <= rect.bottom;
    if (!insideTarget) return -1;

    const style = window.getComputedStyle(target);
    const paddingLeft = parseFloat(style.paddingLeft) || 0;
    const paddingRight = parseFloat(style.paddingRight) || 0;
    const paddingTop = parseFloat(style.paddingTop) || 0;
    const contentWidth = Math.max(rect.width - paddingLeft - paddingRight, 1);
    const contentX = Math.max(0, event.clientX - rect.left - paddingLeft + target.scrollLeft);

    if (target instanceof HTMLInputElement) {
      return Math.round((contentX / contentWidth) * target.value.length);
    }

    const lines = target.value.split("\n");
    const fontSize = parseFloat(style.fontSize) || 13;
    const lineHeight = parseFloat(style.lineHeight) || fontSize * 1.4;
    const contentY = Math.max(0, event.clientY - rect.top - paddingTop + target.scrollTop);
    const lineIndex = Math.min(lines.length - 1, Math.floor(contentY / lineHeight));
    const lineStart = lines.slice(0, lineIndex).reduce((offset, line) => offset + line.length + 1, 0);
    const lineText = lines[lineIndex] ?? "";
    const column = Math.round((contentX / contentWidth) * Math.max(lineText.length, 1));
    return lineStart + Math.min(lineText.length, Math.max(0, column));
  }

  /** @param {Element} target @param {string} text */
  function insertText(target, text) {
    if (!text) return false;
    if (target.hasAttribute("data-app-editable")) {
      const detail = { text, accepted: false };
      target.dispatchEvent(new CustomEvent(APP_CONTEXT_PASTE_EVENT, { detail }));
      return detail.accepted;
    }
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      const start = target.selectionStart ?? target.value.length;
      const end = target.selectionEnd ?? start;
      target.setRangeText(text, start, end, "end");
      target.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: text }));
      target.focus();
      return true;
    }
    if (target instanceof HTMLElement && target.isContentEditable) {
      target.focus();
      return document.execCommand?.("insertText", false, text) ?? false;
    }
    return false;
  }

  function closeMenu() {
    menu = null;
  }

  function notifyMenuOpen() {
    window.dispatchEvent(new CustomEvent(APP_CONTEXT_MENU_OPEN_EVENT, { detail: { source: "app" } }));
  }

  /** @param {MouseEvent} event */
  function handleContextMenu(event) {
    if (!(event.target instanceof Element)) {
      event.preventDefault();
      closeMenu();
      return;
    }

    event.preventDefault();
    const editableTarget = editableTargetFor(event.target);
    const copyText = selectedCustomTextAtPoint(event.target, event)
      || selectedPlainEditableTextAtPoint(editableTarget, event)
      || selectedDomTextAtPoint(event);

    if (!copyText && !editableTarget) {
      closeMenu();
      return;
    }

    notifyMenuOpen();
    menu = {
      ...menuPosition(event),
      copyText,
      editableTarget,
      status: "",
    };
  }

  async function copySelection() {
    if (!menu?.copyText) return;
    try {
      if (!navigator.clipboard?.writeText) throw new Error("Clipboard write unavailable");
      await navigator.clipboard.writeText(menu.copyText);
      closeMenu();
    } catch {
      menu = { ...menu, status: "copy_failed" };
    }
  }

  async function pasteClipboard() {
    if (!menu?.editableTarget) return;
    try {
      const text = await navigator.clipboard?.readText();
      if (!text || !insertText(menu.editableTarget, text)) {
        menu = { ...menu, status: "paste_failed" };
        return;
      }
      closeMenu();
    } catch {
      menu = { ...menu, status: "paste_failed" };
    }
  }

  onMount(() => {
    const onContextMenu = (/** @type {MouseEvent} */ event) => handleContextMenu(event);
    const onPointerDown = () => closeMenu();
    const onScroll = () => closeMenu();
    const onKeydown = (/** @type {KeyboardEvent} */ event) => {
      if (event.key === "Escape") closeMenu();
    };
    const onAppMenuOpen = (/** @type {CustomEvent<{ source?: string }>} */ event) => {
      if (event.detail?.source !== "app") closeMenu();
    };
    window.addEventListener("contextmenu", onContextMenu);
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener(APP_CONTEXT_MENU_OPEN_EVENT, /** @type {EventListener} */ (onAppMenuOpen));
    return () => {
      window.removeEventListener("contextmenu", onContextMenu);
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("keydown", onKeydown, true);
      window.removeEventListener(APP_CONTEXT_MENU_OPEN_EVENT, /** @type {EventListener} */ (onAppMenuOpen));
    };
  });

  onDestroy(() => {
    closeMenu();
  });
</script>

{#if menu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="app-context-menu"
    style={`left: ${menu.x}px; top: ${menu.y}px;`}
    onmousedown={(event) => event.stopPropagation()}
  >
    {#if menu.copyText}
      <button type="button" class="app-context-menu__item" onclick={copySelection}>
        <Clipboard size={14} strokeWidth={1.8} />
        <span>{$t("context_menu.copy")}</span>
      </button>
    {/if}
    {#if menu.editableTarget}
      <button type="button" class="app-context-menu__item" onclick={pasteClipboard}>
        <ClipboardPaste size={14} strokeWidth={1.8} />
        <span>{$t("context_menu.paste")}</span>
      </button>
    {/if}
    {#if menu.status === "copy_failed"}
      <div class="app-context-menu__status">{$t("context_menu.copy_failed")}</div>
    {:else if menu.status === "paste_failed"}
      <div class="app-context-menu__status">{$t("context_menu.paste_failed")}</div>
    {/if}
  </div>
{/if}

/**
 * WKWebView ignores mouse moves in a panel that is not the key window, so
 * `:hover` never matches in the notch. The native cursor tracker streams the
 * pointer instead, and this marks the elements under it with `data-hover`.
 */
let hovered: Element[] = [];

export function hoverAt(point: [number, number] | null) {
  const next: Element[] = [];
  if (point) {
    for (let element = document.elementFromPoint(...point); element; element = element.parentElement) {
      next.push(element);
      if (element.classList.contains("notch")) break;
    }
  }
  for (const element of hovered) {
    if (!next.includes(element)) element.removeAttribute("data-hover");
  }
  for (const element of next) element.setAttribute("data-hover", "");
  hovered = next;
}

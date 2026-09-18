/**
 * WKWebView ignores mouse moves in a panel that is not the key window, so
 * `:hover` never matches in the notch. The native cursor tracker streams the
 * pointer instead, and this marks the elements under it with `data-hover`.
 *
 * The same silence means the webview never updates the mouse cursor either, so
 * the shape it asks for is read back from the stylesheet and set natively.
 */
import { native } from "./native";

let hovered: Element[] = [];
let target: Element | null = null;
let shape = "default";

export function hoverAt(point: [number, number] | null) {
  const next: Element[] = [];
  const top = point ? document.elementFromPoint(...point) : null;
  for (let element = top; element; element = element.parentElement) {
    next.push(element);
    if (element.classList.contains("notch")) break;
  }
  for (const element of hovered) {
    if (!next.includes(element)) element.removeAttribute("data-hover");
  }
  for (const element of next) element.setAttribute("data-hover", "");
  hovered = next;

  // `cursor` is inherited, so the topmost element alone answers the question.
  if (top === target) return;
  target = top;
  const asked = top ? getComputedStyle(top).cursor : "default";
  const wanted = asked === "pointer" || asked === "grab" ? asked : "default";
  if (wanted === shape) return;
  shape = wanted;
  native.setCursor(wanted).catch(() => {});
}

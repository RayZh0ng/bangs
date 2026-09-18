/**
 * Bangs speaks Chinese or English, following the system unless the tray menu
 * pins it to one. Call sites pass both texts rather than a key, so the words
 * stay where they are read: `t("退出", "Quit")`.
 */
// The native side resolves this: WKWebView reports the app's own language,
// not the system's, so `navigator.language` is no help here.
let chinese = true;

/** `"zh"` or `"en"`, as resolved by the native side. */
export function setLanguage(language: string) {
  chinese = language !== "en";
}

export function t(zh: string, en: string) {
  return chinese ? zh : en;
}

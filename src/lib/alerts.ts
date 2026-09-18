import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

let audio: AudioContext | null = null;

/** Creates the audio context during a user gesture so later chimes may play. */
export function primeAudio() {
  try {
    audio ??= new AudioContext();
    void audio.resume();
  } catch {
    // Sound is a nicety; the notch and notification still signal completion.
  }
}

export function playChime() {
  try {
    audio ??= new AudioContext();
    const context = audio;
    void context.resume();
    [880, 1318.5].forEach((frequency, index) => {
      const start = context.currentTime + index * 0.18;
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = "sine";
      oscillator.frequency.value = frequency;
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.18, start + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.001, start + 0.6);
      oscillator.connect(gain).connect(context.destination);
      oscillator.start(start);
      oscillator.stop(start + 0.65);
    });
  } catch {
    // See primeAudio.
  }
}

/** Single soft tone, for background updates that are not the timer. */
export function playPing() {
  try {
    audio ??= new AudioContext();
    const context = audio;
    void context.resume();
    const start = context.currentTime;
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    oscillator.type = "sine";
    oscillator.frequency.value = 1046.5;
    gain.gain.setValueAtTime(0, start);
    gain.gain.linearRampToValueAtTime(0.12, start + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.001, start + 0.45);
    oscillator.connect(gain).connect(context.destination);
    oscillator.start(start);
    oscillator.stop(start + 0.5);
  } catch {
    // See primeAudio.
  }
}

export async function notify(title: string, body: string) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) granted = (await requestPermission()) === "granted";
    if (granted) sendNotification({ title, body });
  } catch {
    // Unbundled dev builds on macOS cannot post notifications.
  }
}

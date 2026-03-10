import type { Page } from "@playwright/test";

export async function installMockSpeechRecognition(page: Page, transcripts: string[]): Promise<void> {
  await page.addInitScript((queuedTranscripts: string[]) => {
    let transcriptIndex = 0;

    class MockSpeechRecognition {
      continuous = true;
      interimResults = true;
      lang = "ko-KR";
      onstart: null | (() => void) = null;
      onend: null | (() => void) = null;
      onerror: null | ((event: unknown) => void) = null;
      onresult: null | ((event: unknown) => void) = null;

      start() {
        if (this.onstart) this.onstart();
        const transcript = queuedTranscripts[Math.min(transcriptIndex, queuedTranscripts.length - 1)] ?? "";
        transcriptIndex += 1;
        setTimeout(() => {
          if (this.onresult) {
            this.onresult({
              resultIndex: 0,
              results: {
                0: {
                  0: { transcript },
                  isFinal: true,
                  length: 1
                },
                length: 1
              }
            });
          }
          if (this.onend) this.onend();
        }, 50);
      }

      stop() {
        if (this.onend) this.onend();
      }
    }

    const speechWindow = window as Window & {
      SpeechRecognition?: unknown;
      webkitSpeechRecognition?: unknown;
    };
    speechWindow.SpeechRecognition = MockSpeechRecognition;
    speechWindow.webkitSpeechRecognition = MockSpeechRecognition;
  }, transcripts);
}

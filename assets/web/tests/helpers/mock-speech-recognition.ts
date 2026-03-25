import type { Page } from "@playwright/test";

type MockSpeechRecognitionResultInput =
  | string
  | {
      transcript: string;
      isFinal?: boolean;
    };

type MockSpeechRecognitionEmission = string | MockSpeechRecognitionResultInput[];

export async function installMockSpeechRecognition(
  page: Page,
  transcripts: MockSpeechRecognitionEmission[]
): Promise<void> {
  await page.addInitScript((queuedTranscripts: MockSpeechRecognitionEmission[]) => {
    let transcriptIndex = 0;

    function toResults(entry: MockSpeechRecognitionEmission): Array<{ transcript: string; isFinal: boolean }> {
      if (!Array.isArray(entry)) {
        return [{ transcript: entry, isFinal: true }];
      }
      return entry.map((value) =>
        typeof value === "string" ? { transcript: value, isFinal: true } : { transcript: value.transcript, isFinal: value.isFinal !== false }
      );
    }

    class MockSpeechRecognition {
      continuous = true;
      interimResults = true;
      lang = "ko-KR";
      private started = false;
      onstart: null | (() => void) = null;
      onend: null | (() => void) = null;
      onerror: null | ((event: unknown) => void) = null;
      onresult: null | ((event: unknown) => void) = null;

      start() {
        this.started = true;
        if (this.onstart) this.onstart();
        const transcriptEntry = queuedTranscripts[Math.min(transcriptIndex, queuedTranscripts.length - 1)] ?? "";
        transcriptIndex += 1;
        setTimeout(() => {
          if (this.onresult) {
            const results = toResults(transcriptEntry);
            const eventResults = results.reduce<Record<number, { 0: { transcript: string }; isFinal: boolean; length: number }> & { length: number }>(
              (acc, result, index) => {
                acc[index] = {
                  0: { transcript: result.transcript },
                  isFinal: result.isFinal,
                  length: 1
                };
                return acc;
              },
              { length: results.length }
            );
            this.onresult({
              resultIndex: 0,
              results: eventResults
            });
          }
        }, 50);
      }

      stop() {
        if (!this.started) return;
        this.started = false;
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

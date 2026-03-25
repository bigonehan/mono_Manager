import * as React from "react";

type SpeechRecognitionAlternativeLike = {
  transcript: string;
};

type SpeechRecognitionResultLike = {
  isFinal: boolean;
  length: number;
  [index: number]: SpeechRecognitionAlternativeLike;
};

type SpeechRecognitionEventLike = {
  resultIndex: number;
  results: ArrayLike<SpeechRecognitionResultLike>;
};

type SpeechRecognitionErrorEventLike = {
  error: string;
};

type BrowserSpeechRecognition = {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  onstart: (() => void) | null;
  onend: (() => void) | null;
  onerror: ((event: SpeechRecognitionErrorEventLike) => void) | null;
  onresult: ((event: SpeechRecognitionEventLike) => void) | null;
  start: () => void;
  stop: () => void;
};

type BrowserSpeechRecognitionConstructor = new () => BrowserSpeechRecognition;

declare global {
  interface Window {
    SpeechRecognition?: BrowserSpeechRecognitionConstructor;
    webkitSpeechRecognition?: BrowserSpeechRecognitionConstructor;
  }
}

type VoiceTargetElement = HTMLInputElement | HTMLTextAreaElement;

function getSpeechRecognitionConstructor(): BrowserSpeechRecognitionConstructor | null {
  if (typeof window === "undefined") return null;
  return window.SpeechRecognition ?? window.webkitSpeechRecognition ?? null;
}

function getSpeechErrorMessage(error: string): string {
  if (error === "not-allowed" || error === "service-not-allowed") {
    return "microphone permission is required";
  }
  if (error === "audio-capture") {
    return "microphone was not found";
  }
  if (error === "no-speech") {
    return "no speech detected";
  }
  if (error === "network") {
    return "voice input network error";
  }
  return "voice input failed";
}

function getJoinedTranscript(baseValue: string, nextTranscript: string): string {
  return [baseValue.trim(), nextTranscript.trim()].filter(Boolean).join(" ").trim();
}

function normalizeTranscript(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

function getTranscriptOverlap(current: string, next: string): number {
  const maxLength = Math.min(current.length, next.length);
  for (let length = maxLength; length > 0; length -= 1) {
    if (current.slice(-length) === next.slice(0, length)) {
      return length;
    }
  }
  return 0;
}

function mergeTranscriptSegments(segments: string[]): string {
  let merged = "";
  for (const segment of segments) {
    const next = normalizeTranscript(segment);
    if (!next) continue;
    if (!merged) {
      merged = next;
      continue;
    }
    if (merged.endsWith(next)) {
      continue;
    }
    const overlap = getTranscriptOverlap(merged, next);
    merged = normalizeTranscript(`${merged}${next.slice(overlap)}`);
  }
  return merged;
}

function setElementValue(element: VoiceTargetElement, nextValue: string): void {
  const prototype =
    element instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  const descriptor = Object.getOwnPropertyDescriptor(prototype, "value");
  if (descriptor?.set) {
    descriptor.set.call(element, nextValue);
  } else {
    element.value = nextValue;
  }
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

export function createVoiceFieldLabel(rawLabel?: string): string {
  const label = String(rawLabel ?? "").trim();
  return label.length > 0 ? label : "text field";
}

export function useVoiceInput<T extends VoiceTargetElement>({
  elementRef,
  label,
  disabled = false
}: {
  elementRef: React.RefObject<T | null>;
  label?: string;
  disabled?: boolean;
}) {
  const recognitionRef = React.useRef<BrowserSpeechRecognition | null>(null);
  const baseValueRef = React.useRef("");
  const pendingTranscriptRef = React.useRef("");
  const commitOnEndRef = React.useRef(false);
  const [supported, setSupported] = React.useState(false);
  const [listening, setListening] = React.useState(false);
  const [error, setError] = React.useState("");

  React.useEffect(() => {
    const Recognition = getSpeechRecognitionConstructor();
    setSupported(Boolean(Recognition));
    if (!Recognition) {
      recognitionRef.current = null;
      return;
    }

    const recognition = new Recognition();
    recognition.continuous = true;
    recognition.interimResults = true;
    recognition.lang = "ko-KR";
    recognition.onstart = () => {
      setListening(true);
      setError("");
      pendingTranscriptRef.current = "";
      commitOnEndRef.current = false;
    };
    recognition.onend = () => {
      setListening(false);
      const element = elementRef.current;
      if (commitOnEndRef.current && element) {
        const pending = normalizeTranscript(pendingTranscriptRef.current);
        if (pending.length > 0) {
          setElementValue(element, getJoinedTranscript(baseValueRef.current, pending));
        }
      }
      pendingTranscriptRef.current = "";
      commitOnEndRef.current = false;
    };
    recognition.onerror = (event) => {
      setError(getSpeechErrorMessage(event.error));
      setListening(false);
      pendingTranscriptRef.current = "";
      commitOnEndRef.current = false;
    };
    recognition.onresult = (event) => {
      const segments: string[] = [];
      for (let index = 0; index < event.results.length; index += 1) {
        const transcript = event.results[index]?.[0]?.transcript ?? "";
        if (normalizeTranscript(transcript).length === 0) continue;
        segments.push(transcript);
      }
      pendingTranscriptRef.current = mergeTranscriptSegments(segments);
    };
    recognitionRef.current = recognition;

    return () => {
      recognition.onstart = null;
      recognition.onend = null;
      recognition.onerror = null;
      recognition.onresult = null;
      try {
        recognition.stop();
      } catch {
        // ignore cleanup stop failures
      }
      recognitionRef.current = null;
    };
  }, [elementRef]);

  React.useEffect(() => {
    if (!disabled || !listening) return;
    commitOnEndRef.current = false;
    try {
      recognitionRef.current?.stop();
    } catch {
      // ignore disable stop failures
    }
  }, [disabled, listening]);

  const toggle = React.useCallback(() => {
    const recognition = recognitionRef.current;
    const element = elementRef.current;
    if (!recognition || !element) {
      setError("voice input is not available");
      return;
    }
    if (listening) {
      commitOnEndRef.current = true;
      recognition.stop();
      return;
    }
    baseValueRef.current = element.value ?? "";
    pendingTranscriptRef.current = "";
    commitOnEndRef.current = false;
    setError("");
    try {
      recognition.lang =
        typeof navigator !== "undefined" && navigator.language?.trim().length > 0 ? navigator.language : "ko-KR";
      recognition.start();
    } catch {
      setError("voice input could not start");
      setListening(false);
    }
  }, [elementRef, listening]);

  return {
    supported,
    listening,
    error,
    buttonDisabled: disabled || !supported,
    buttonLabel: `${listening ? "stop" : "record"} voice input for ${createVoiceFieldLabel(label)}`,
    statusText: error || (supported ? (listening ? "listening" : "voice input ready") : "voice input unsupported"),
    toggle
  };
}

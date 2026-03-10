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
  const transcriptRef = React.useRef("");
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
    };
    recognition.onend = () => {
      setListening(false);
    };
    recognition.onerror = (event) => {
      setError(getSpeechErrorMessage(event.error));
      setListening(false);
    };
    recognition.onresult = (event) => {
      let interim = "";
      for (let index = event.resultIndex; index < event.results.length; index += 1) {
        const result = event.results[index];
        const transcript = result[0]?.transcript?.trim() ?? "";
        if (!transcript) continue;
        if (result.isFinal) {
          transcriptRef.current = [transcriptRef.current, transcript].filter(Boolean).join(" ").trim();
        } else {
          interim = [interim, transcript].filter(Boolean).join(" ").trim();
        }
      }
      const element = elementRef.current;
      if (!element) return;
      setElementValue(
        element,
        getJoinedTranscript(baseValueRef.current, [transcriptRef.current, interim].filter(Boolean).join(" ").trim())
      );
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
      recognition.stop();
      return;
    }
    baseValueRef.current = element.value ?? "";
    transcriptRef.current = "";
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

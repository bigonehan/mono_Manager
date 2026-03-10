import * as React from "react";
import { Mic, MicOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { createVoiceFieldLabel, useVoiceInput } from "@/lib/use-voice-input";

type TextareaProps = React.TextareaHTMLAttributes<HTMLTextAreaElement> & {
  "data-testid"?: string;
  voiceLabel?: string;
  voiceInputDisabled?: boolean;
};

function assignRef<T>(ref: React.ForwardedRef<T>, value: T | null): void {
  if (typeof ref === "function") {
    ref(value);
    return;
  }
  if (ref) {
    ref.current = value;
  }
}

export const Textarea = React.forwardRef<
  HTMLTextAreaElement,
  TextareaProps
>(({ className, voiceLabel, voiceInputDisabled, disabled, ...props }, ref) => {
  const innerRef = React.useRef<HTMLTextAreaElement | null>(null);
  const setRef = React.useCallback(
    (node: HTMLTextAreaElement | null) => {
      innerRef.current = node;
      assignRef(ref, node);
    },
    [ref]
  );
  const textareaTestId = typeof props["data-testid"] === "string" ? props["data-testid"] : undefined;
  const voice = useVoiceInput({
    elementRef: innerRef,
    label: voiceLabel ?? props["aria-label"] ?? props.placeholder ?? props.name ?? props.id,
    disabled: Boolean(voiceInputDisabled || disabled)
  });

  return (
    <div className="flex w-full min-w-0 items-start gap-2">
      <textarea
        ref={setRef}
        disabled={disabled}
        className={cn(
          "min-h-[80px] w-full min-w-0 rounded-md border border-input bg-card px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          className
        )}
        {...props}
      />
      <Button
        type="button"
        size="icon"
        variant={voice.listening ? "default" : "outline"}
        className="mt-1 shrink-0"
        data-testid={textareaTestId ? `${textareaTestId}-voice` : undefined}
        aria-label={voice.buttonLabel}
        title={voice.statusText}
        disabled={voice.buttonDisabled}
        onClick={voice.toggle}
      >
        {voice.listening ? <MicOff className="h-4 w-4" /> : <Mic className="h-4 w-4" />}
        <span className="sr-only">
          {createVoiceFieldLabel(voiceLabel ?? props["aria-label"] ?? props.placeholder ?? props.name ?? props.id)}
        </span>
      </Button>
    </div>
  );
});
Textarea.displayName = "Textarea";

import * as React from "react";
import { Mic, MicOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { createVoiceFieldLabel, useVoiceInput } from "@/lib/use-voice-input";

type InputProps = React.InputHTMLAttributes<HTMLInputElement> & {
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

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, voiceLabel, voiceInputDisabled, type, disabled, ...props }, ref) => {
    const innerRef = React.useRef<HTMLInputElement | null>(null);
    const setRef = React.useCallback(
      (node: HTMLInputElement | null) => {
        innerRef.current = node;
        assignRef(ref, node);
      },
      [ref]
    );
    const inputTestId = typeof props["data-testid"] === "string" ? props["data-testid"] : undefined;
    const voice = useVoiceInput({
      elementRef: innerRef,
      label: voiceLabel ?? props["aria-label"] ?? props.placeholder ?? props.name ?? props.id,
      disabled:
        Boolean(voiceInputDisabled || disabled) ||
        ["checkbox", "radio", "file", "hidden", "range", "color", "date", "datetime-local", "month", "time", "week"].includes(
          type ?? "text"
        )
    });

    return (
      <div className="flex w-full min-w-0 items-center gap-2">
        <input
          ref={setRef}
          type={type}
          disabled={disabled}
          className={cn(
            "flex h-9 w-full min-w-0 rounded-md border border-input bg-card px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            className
          )}
          {...props}
        />
        <Button
          type="button"
          size="icon"
          variant={voice.listening ? "default" : "outline"}
          className="shrink-0"
          data-testid={inputTestId ? `${inputTestId}-voice` : undefined}
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
  }
);
Input.displayName = "Input";

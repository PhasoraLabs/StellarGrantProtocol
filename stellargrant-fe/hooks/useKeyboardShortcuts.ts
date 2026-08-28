import { useEffect, useRef } from "react";

export type ShortcutDefinition = {
  key: string;
  description: string;
  action: (e?: KeyboardEvent) => void;
  condition?: () => boolean;
};

export function useKeyboardShortcuts(shortcuts: ShortcutDefinition[]) {
  const sequenceRef = useRef<string[]>([]);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;

      // Ignore if user is typing in an input field
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT" ||
        target.isContentEditable
      ) {
        return;
      }

      // Ignore modifier keys alone
      if (event.key === "Shift" || event.key === "Control" || event.key === "Alt" || event.key === "Meta") {
        return;
      }

      // Add the new key to the sequence
      // Use event.key directly, which handles case sensitivity (e.g. 'F' vs 'f')
      sequenceRef.current.push(event.key);
      const currentSequenceStr = sequenceRef.current.join(" ");

      // Check if there is an exact match
      const exactMatch = shortcuts.find(
        (s) => s.key === currentSequenceStr && (s.condition ? s.condition() : true)
      );

      if (exactMatch) {
        event.preventDefault();
        exactMatch.action(event);
        sequenceRef.current = [];
        if (timeoutRef.current) {
          clearTimeout(timeoutRef.current);
          timeoutRef.current = null;
        }
        return;
      }

      // Check if there is a partial match (prefix)
      const isPrefix = shortcuts.some((s) => s.key.startsWith(currentSequenceStr + " "));

      if (isPrefix) {
        // Wait for more keys
        if (timeoutRef.current) {
          clearTimeout(timeoutRef.current);
        }
        timeoutRef.current = setTimeout(() => {
          sequenceRef.current = [];
        }, 1000);
      } else {
        // Not a prefix for any shortcut. Reset and start over with just this key,
        // in case the user typed an invalid key followed by a valid single-key shortcut.
        const singleKeyStr = event.key;
        const singleMatch = shortcuts.find(
          (s) => s.key === singleKeyStr && (s.condition ? s.condition() : true)
        );

        if (singleMatch && sequenceRef.current.length > 1) {
           event.preventDefault();
           singleMatch.action(event);
           sequenceRef.current = [];
           if (timeoutRef.current) {
             clearTimeout(timeoutRef.current);
             timeoutRef.current = null;
           }
        } else {
           sequenceRef.current = [];
           if (timeoutRef.current) {
             clearTimeout(timeoutRef.current);
             timeoutRef.current = null;
           }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, [shortcuts]);
}

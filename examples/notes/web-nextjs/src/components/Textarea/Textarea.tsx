import { FC, useRef, SyntheticEvent, useEffect } from "react";

type EditState = {
  selectionEnd: number;
  length: number;
  text: string;
};

export type SelectEvent = {
  start: number;
  end: number;
};

export type ChangeEvent = {
  start: number;
  end: number;
  text: string;
};

interface TextareaProps {
  value: string;
  selectionStart: number;
  selectionEnd: number;
  onSelect: (selection: SelectEvent) => void;
  onChange: (change: ChangeEvent) => void;
  className: string;
}

// We need to monitor these for performance. If this becomes a problem
// the codepoint offset <-> byte offset conversion can be done in the
// core and exposed in the interface

function ucToBytes(value: string, index: number): number {
  return Array.from(value).slice(0, index).join("").length;
}

function bytesToUc(value: string, index: number): number {
  return Array.from(value.substring(0, index)).length;
}

const Textarea: FC<TextareaProps> = ({
  value,
  selectionStart,
  selectionEnd,
  onSelect,
  onChange,
  className,
}) => {
  const taRef = useRef(null);
  useEffect(() => {
    if (taRef.current == null) return;
    let ta: HTMLInputElement = taRef.current;

    // Convert selection start and end to byte offsets

    ta.selectionStart = ucToBytes(value, selectionStart);
    ta.selectionEnd = ucToBytes(value, selectionEnd);

    ta.addEventListener("beforeinput", onBeforeInput);
    ta.addEventListener("input", onInput);

    return () => {
      ta.removeEventListener("beforeinput", onBeforeInput);
      ta.removeEventListener("input", onInput);
    };
  });

  const beforeEdit = useRef<EditState>({
    selectionEnd: 0,
    length: 0,
    text: "",
  });

  const onBeforeInput = (event: InputEvent) => {
    if (event.target == null) return;
    let target = event.target as HTMLInputElement;

    // This may become quite painful.
    let text = event.data ?? (event.inputType == "insertLineBreak" ? "\n" : "");

    beforeEdit.current.selectionEnd = target.selectionEnd ?? 0;
    beforeEdit.current.length = target.value.length;
    beforeEdit.current.text = text;
  };

  const onInput = (event: Event): any => {
    if (event.target == null) return;
    let target = event.target as HTMLInputElement;

    let length = target.value.length;
    let end = beforeEdit.current.selectionEnd;
    let text = beforeEdit.current.text;

    let removed = text.length - length + beforeEdit.current.length;
    let start = end - removed;

    onChange({
      start: bytesToUc(value, start),
      end: bytesToUc(value, end),
      text,
    });
  };

  const localOnSelect = (event: SyntheticEvent) => {
    if (event.target == null) return;
    let target = event.target as HTMLInputElement;

    let end = target.selectionEnd ?? 0;
    let start = target.selectionStart ?? 0;

    beforeEdit.current.selectionEnd = end;
    beforeEdit.current.length = target.value.length;

    onSelect({ start: bytesToUc(value, start), end: bytesToUc(value, end) });
  };

  return (
    <textarea
      className={className}
      ref={taRef}
      onSelect={localOnSelect}
      onChange={() => {}}
      value={value}
    />
  );
};

export default Textarea;

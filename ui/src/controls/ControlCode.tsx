import type { ControlProps } from "./registry";
export function ControlCode({ id, value, onChange, disabled, required, field }: ControlProps) {
  // Monaco Editor would go here in Phase 3 — native textarea fallback for now
  return (
    <textarea
      id={id}
      className="form-input form-code"
      value={String(value ?? "")}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      required={required}
      rows={10}
      spellCheck={false}
      data-lang={field.options}
    />
  );
}

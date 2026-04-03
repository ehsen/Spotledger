import type { ControlProps } from "./registry";
export function ControlText({ id, value, onChange, disabled, required }: ControlProps) {
  return (
    <textarea
      id={id}
      className="form-input form-textarea"
      value={String(value ?? "")}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      required={required}
      rows={4}
    />
  );
}

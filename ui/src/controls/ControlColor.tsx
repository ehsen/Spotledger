import type { ControlProps } from "./registry";
export function ControlColor({ id, value, onChange, disabled }: ControlProps) {
  return (
    <input
      id={id}
      type="color"
      className="form-input form-color"
      value={String(value ?? "#000000")}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
    />
  );
}

import type { ControlProps } from "./registry";
export function ControlDatetime({ id, value, onChange, disabled, required }: ControlProps) {
  // Frappe stores datetimes as "YYYY-MM-DD HH:MM:SS", HTML expects "YYYY-MM-DDTHH:MM"
  const htmlVal = String(value ?? "").replace(" ", "T").slice(0, 16);
  function handleChange(e: React.ChangeEvent<HTMLInputElement>) {
    const v = e.target.value; // "YYYY-MM-DDTHH:MM"
    onChange(v ? v.replace("T", " ") + ":00" : null);
  }
  return (
    <input
      id={id}
      type="datetime-local"
      className="form-input form-datetime"
      value={htmlVal}
      onChange={handleChange}
      disabled={disabled}
      required={required}
    />
  );
}

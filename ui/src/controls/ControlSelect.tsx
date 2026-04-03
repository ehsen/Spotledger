import type { ControlProps } from "./registry";

export function ControlSelect({ id, value, onChange, disabled, required, field, options: runtimeOptions }: ControlProps) {
  // Options can be runtime-overridden via field_states or come from field metadata
  const rawOptions = runtimeOptions ?? (field.options ?? "").split("\n").filter(Boolean);
  const opts = rawOptions.map((o) => String(o).trim()).filter(Boolean);

  return (
    <select
      id={id}
      className="form-input form-select"
      value={String(value ?? "")}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      required={required}
    >
      <option value="">—</option>
      {opts.map((o) => (
        <option key={o} value={o}>{o}</option>
      ))}
    </select>
  );
}

// FormField — renders a single field using the controls registry

import { useCallback, memo } from "react";
import type { DocField } from "../types/doctype";
import type { FieldState } from "../types/form";
import { get_control } from "../controls/registry";
import { is_field_visible } from "./visibility";
import { useFormStore } from "../naja/store/store-context";

interface Props {
  field: DocField;
  onFieldChange: (fieldname: string, value: unknown) => void;
}

export const FormField = memo(function FormField({ field, onFieldChange }: Props) {
  const useStore = useFormStore();
  const doc = useStore((s) => s.doc);
  const field_states = useStore((s) => s.field_states);

  const fstate: FieldState = field_states[field.fieldname] ?? {};

  // Visibility check — includes dependencies and programmatic hide
  const visible = is_field_visible(
    doc,
    field.visible_if,
    field.depends_on,
    fstate.hidden,
  );

  if (!visible) return null;

  const value = doc[field.fieldname] ?? null;
  const disabled = !!(fstate.disabled ?? field.read_only);
  const required = !!(fstate.required ?? field.reqd);
  const description = fstate.description ?? field.description;
  const error = fstate.error;

  const handleChange = useCallback(
    (val: unknown) => onFieldChange(field.fieldname, val),
    [field.fieldname, onFieldChange],
  );

  const Control = get_control(field.fieldtype);

  return (
    <div
      className={`form-field form-field-${field.fieldtype.toLowerCase().replace(/\s+/g, "-")} ${error ? "has-error" : ""}`}
      style={field.columns ? { gridColumn: `span ${field.columns}` } : undefined}
    >
      {field.fieldtype !== "Check" && (
        <label className={`form-field-label ${required ? "required" : ""}`} htmlFor={`field-${field.fieldname}`}>
          {field.label}
          {required && <span className="required-star" aria-hidden="true"> *</span>}
        </label>
      )}
      <Control
        id={`field-${field.fieldname}`}
        field={field}
        value={value}
        onChange={handleChange}
        disabled={disabled}
        required={required}
        options={field.fieldtype === "Select" ? (fstate.options ?? undefined) : undefined}
      />
      {description && <p className="form-field-desc">{description}</p>}
      {error && <p className="form-field-error">{error}</p>}
    </div>
  );
});

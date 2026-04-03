// Controls registry — fieldtype → React component map
// This is the ONLY file in the project that imports Syncfusion directly.
// All other code interacts through this registry.

import type { ComponentType } from "react";
import type { DocField, FieldType } from "../types/doctype";
import { ControlData } from "./ControlData";
import { ControlText } from "./ControlText";
import { ControlInt } from "./ControlInt";
import { ControlFloat } from "./ControlFloat";
import { ControlCurrency } from "./ControlCurrency";
import { ControlCheck } from "./ControlCheck";
import { ControlSelect } from "./ControlSelect";
import { ControlLink } from "./ControlLink/ControlLink";
import { ControlDate } from "./ControlDate";
import { ControlDatetime } from "./ControlDatetime";
import { ControlTable } from "./ControlTable/ControlTable";
import { ControlReadOnly } from "./ControlReadOnly";
import { ControlAttach } from "./ControlAttach";
import { ControlPassword } from "./ControlPassword";
import { ControlCode } from "./ControlCode";
import { ControlPercent } from "./ControlPercent";
import { ControlColor } from "./ControlColor";
import { ControlRating } from "./ControlRating";

export interface ControlProps {
  id?: string;
  field: DocField;
  value: unknown;
  onChange: (value: unknown) => void;
  disabled?: boolean;
  required?: boolean;
  options?: string[];   // runtime override for Select options
}

// The null control — renders nothing (used for layout-only field types)
function NullControl() { return null; }

// Fallback for unmapped field types
function FallbackControl({ field, value, onChange, disabled }: ControlProps) {
  return (
    <input
      type="text"
      className="form-input"
      value={String(value ?? "")}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      data-fieldtype={field.fieldtype}
    />
  );
}

const registry = new Map<string, ComponentType<ControlProps>>();

const defaults: [FieldType | string, ComponentType<ControlProps>][] = [
  // Text variants
  ["Data",            ControlData],
  ["Small Text",      ControlText],
  ["Text",            ControlText],
  ["Long Text",       ControlText],
  ["Text Editor",     ControlText],    // TODO: RichText in Phase 3
  ["Code",            ControlCode],
  ["Password",        ControlPassword],
  // Numeric
  ["Int",             ControlInt],
  ["Float",           ControlFloat],
  ["Currency",        ControlCurrency],
  ["Percent",         ControlPercent],
  ["Rating",          ControlRating],
  // Checkboxes / Select
  ["Check",           ControlCheck],
  ["Select",          ControlSelect],
  ["MultiSelect",     ControlSelect],  // TODO: MultiSelect control
  // Links
  ["Link",            ControlLink],
  ["Dynamic Link",    ControlLink],    // TODO: Dynamic link variant
  // Date / Time
  ["Date",            ControlDate],
  ["Datetime",        ControlDatetime],
  ["Time",            ControlDate],    // TODO: TimePicker
  // Table
  ["Table",           ControlTable],
  ["Table MultiSelect", ControlTable],
  // Files
  ["Attach",          ControlAttach],
  ["Attach Image",    ControlAttach],
  // Color
  ["Color",           ControlColor],
  // Read-only / display
  ["Read Only",       ControlReadOnly],
  ["HTML",            ControlReadOnly],
  ["Heading",         ControlReadOnly],
  ["Button",          NullControl as ComponentType<ControlProps>],
  // Layout — handled by FormLayout, never reach the registry
  ["Section Break",   NullControl as ComponentType<ControlProps>],
  ["Column Break",    NullControl as ComponentType<ControlProps>],
  ["Tab Break",       NullControl as ComponentType<ControlProps>],
  ["Barcode",         ControlData],
  ["Duration",        ControlData],
  ["Geolocation",     FallbackControl],
  ["Signature",       FallbackControl],
];

for (const [type, component] of defaults) {
  registry.set(type, component);
}

export function get_control(fieldtype: string): ComponentType<ControlProps> {
  return registry.get(fieldtype) ?? FallbackControl;
}

export function register_control(fieldtype: string, component: ComponentType<ControlProps>) {
  registry.set(fieldtype, component);
}

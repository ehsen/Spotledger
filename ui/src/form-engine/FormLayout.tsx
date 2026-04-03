// FormLayout — Section/Column/Tab layout engine
// Converts the flat DocType field list into a structured layout.

import { useState } from "react";
import type { DocField } from "../types/doctype";
import { FormField } from "./FormField";

interface Props {
  fields: DocField[];
  onFieldChange: (fieldname: string, value: unknown) => void;
}

interface LayoutSection {
  label?: string;
  collapsible?: boolean;
  columns: LayoutColumn[];
}

interface LayoutColumn {
  fields: DocField[];
}

/*
  Layout building algorithm:
  - Walk fields top to bottom
  - Section Break → start new section
  - Column Break → advance to next column
  - Tab Break → start a tab group (handled at a higher level for now — rendered as sections)
  - Other fields → add to current column
*/
function build_layout(fields: DocField[]): LayoutSection[] {
  const sections: LayoutSection[] = [];
  let current_section: LayoutSection = { columns: [{ fields: [] }] };
  let current_col = 0;

  for (const field of fields) {
    if (field.fieldtype === "Section Break") {
      if (
        current_section.columns.some((c) => c.fields.length > 0) ||
        current_section.label
      ) {
        sections.push(current_section);
      }
      current_section = {
        label: field.label,
        collapsible: !!field.collapsible,
        columns: [{ fields: [] }],
      };
      current_col = 0;
    } else if (field.fieldtype === "Column Break") {
      current_col++;
      if (!current_section.columns[current_col]) {
        current_section.columns.push({ fields: [] });
      }
    } else if (field.fieldtype === "Tab Break") {
      // Treat Tab Break the same as Section Break for now
      if (current_section.columns.some((c) => c.fields.length > 0)) {
        sections.push(current_section);
      }
      current_section = {
        label: field.label,
        collapsible: false,
        columns: [{ fields: [] }],
      };
      current_col = 0;
    } else {
      if (!current_section.columns[current_col]) {
        current_section.columns.push({ fields: [] });
      }
      current_section.columns[current_col].fields.push(field);
    }
  }

  // Push last section
  if (current_section.columns.some((c) => c.fields.length > 0)) {
    sections.push(current_section);
  }

  return sections;
}

export function FormLayout({ fields, onFieldChange }: Props) {
  const sections = build_layout(fields);

  return (
    <div className="form-layout">
      {sections.map((section, si) => (
        <FormSection
          key={si}
          section={section}
          onFieldChange={onFieldChange}
        />
      ))}
    </div>
  );
}

function FormSection({
  section,
  onFieldChange,
}: {
  section: LayoutSection;
  onFieldChange: (f: string, v: unknown) => void;
}) {
  const [collapsed, setCollapsed] = useState(false);

  const header = section.label || section.collapsible;

  return (
    <section className={`form-section ${collapsed ? "collapsed" : ""}`}>
      {header && (
        <div
          className="form-section-header"
          onClick={section.collapsible ? () => setCollapsed((c) => !c) : undefined}
          style={section.collapsible ? { cursor: "pointer" } : undefined}
        >
          {section.collapsible && (
            <span className={`section-chevron ${collapsed ? "is-collapsed" : ""}`}>›</span>
          )}
          {section.label && <span className="section-label">{section.label}</span>}
        </div>
      )}
      {!collapsed && (
        <div
          className="form-columns"
          style={{ gridTemplateColumns: `repeat(${section.columns.length}, 1fr)` }}
        >
          {section.columns.map((col, ci) => (
            <div key={ci} className="form-column">
              {col.fields.map((field) => (
                <FormField
                  key={field.fieldname}
                  field={field}
                  onFieldChange={onFieldChange}
                />
              ))}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

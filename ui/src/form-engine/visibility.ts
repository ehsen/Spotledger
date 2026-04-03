// Visibility rules engine — typed rule evaluation
// Replaces Frappe's depends_on: "eval:..." with structured rule objects.

import type { VisibilityRule, SimpleRule, CompoundRule } from "../types/doctype";
import type { DocRecord } from "../types/form";

function eval_simple(rule: SimpleRule, doc: DocRecord): boolean {
  const val = doc[rule.field];
  switch (rule.op) {
    case "eq":      return val == rule.value; // intentional == for type coercion
    case "ne":      return val != rule.value;
    case "gt":      return Number(val) > Number(rule.value);
    case "lt":      return Number(val) < Number(rule.value);
    case "gte":     return Number(val) >= Number(rule.value);
    case "lte":     return Number(val) <= Number(rule.value);
    case "in":      return Array.isArray(rule.value) && rule.value.includes(val);
    case "not_in":  return Array.isArray(rule.value) && !rule.value.includes(val);
    case "is_set":  return val !== null && val !== undefined && val !== "";
    case "is_empty":return val === null || val === undefined || val === "";
  }
}

function eval_rule(rule: VisibilityRule, doc: DocRecord): boolean {
  if ("op" in rule) return eval_simple(rule, doc);
  const compound = rule as CompoundRule;
  if (compound.all) return compound.all.every((r) => eval_rule(r, doc));
  if (compound.any) return compound.any.some((r) => eval_rule(r, doc));
  return true;
}

// Frappe depends_on string patterns we handle without eval()
// e.g. "eval:doc.x == 1", "eval:doc.x", "doc.x"
function parse_frappe_expr(expr: string, doc: DocRecord): boolean {
  const s = expr.replace(/^eval:/, "").trim();
  // "doc.field" — truthy check
  const simple_field = /^doc\.(\w+)$/.exec(s);
  if (simple_field) return !!(doc[simple_field[1]]);

  // "doc.field == value" or "doc.field != value"
  const eq = /^doc\.(\w+)\s*(==|!=|===|!==|>=|<=|>|<)\s*(.+)$/.exec(s);
  if (eq) {
    const field_val = doc[eq[1]];
    const op = eq[2] as string;
    let rhs_raw = eq[3].trim().replace(/^["']|["']$/g, ""); // strip quotes
    // handle numeric rhs
    const rhs: unknown = isNaN(Number(rhs_raw)) ? rhs_raw : Number(rhs_raw);
    if (op === "==" || op === "===") return field_val == rhs;
    if (op === "!=" || op === "!==") return field_val != rhs;
    if (op === ">")  return Number(field_val) > Number(rhs);
    if (op === "<")  return Number(field_val) < Number(rhs);
    if (op === ">=") return Number(field_val) >= Number(rhs);
    if (op === "<=") return Number(field_val) <= Number(rhs);
  }

  // "in_list(doc.field, 'a,b,c')"
  const in_list = /^in_list\(doc\.(\w+),\s*["']([^"']+)["']\)/.exec(s);
  if (in_list) {
    const val = String(doc[in_list[1]] ?? "");
    return in_list[2].split(",").map((v) => v.trim()).includes(val);
  }

  // Fallback: be permissive — show the field
  return true;
}

/**
 * Evaluate whether a field should be visible given the current doc state.
 * Uses typed visible_if rule if present; falls back to Frappe depends_on string.
 */
export function is_field_visible(
  doc: DocRecord,
  visible_if?: VisibilityRule,
  depends_on?: string,
  field_state_hidden?: boolean,
): boolean {
  // Explicit programmatic hide from a hook takes priority
  if (field_state_hidden) return false;

  if (visible_if) return eval_rule(visible_if, doc);
  if (depends_on) return parse_frappe_expr(depends_on, doc);
  return true;
}

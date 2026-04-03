// Tab type definitions and FlexLayout model factory
import { Model } from "flexlayout-react";

export type TabKind = "list" | "form" | "welcome" | "ai";

export interface TabDescriptor {
  id: string;
  kind: TabKind;
  doctype?: string;
  docname?: string;    // undefined = new doc
  title: string;
  dirty?: boolean;
}

/** Create the initial FlexLayout model with a single welcome tab. */
export function create_initial_model(): Model {
  return Model.fromJson({
    global: {
      tabEnableRename: false,
      tabSetEnableDeleteWhenEmpty: false,
      tabSetMinWidth: 100,
      tabSetMinHeight: 100,
      borderMinSize: 100,
      splitterSize: 4,
    },
    borders: [],
    layout: {
      type: "row",
      weight: 100,
      children: [
        {
          type: "tabset",
          weight: 100,
          id: "main-tabset",
          children: [
            {
              type: "tab",
              id: "welcome",
              name: "Welcome",
              component: "welcome",
            },
          ],
        },
      ],
    },
  });
}

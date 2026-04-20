-- ============================================================================
-- Spotledger Workflow Functions
-- ============================================================================
--
-- Graph-based workflow state machine.  All workflow logic lives here so that
-- the Rust handlers become thin wrappers that make a single DB round-trip.
--
-- Tables used:
--   tabWorkflow             – document_type, is_active, workflow_state_field
--   tabWorkflowTransition   – parent (workflow name), state, action, next_state, allowed
--   tabWorkflowDocumentState – parent (workflow name), state, doc_status
-- ============================================================================

-- ----------------------------------------------------------------------------
-- fn::workflow::get_transitions($user, $doctype, $current_state) → array
--
-- Returns all visible workflow transitions from $current_state for $doctype.
-- Matches current Rust behaviour: all transitions from this state regardless
-- of user roles (role-gating can be layered on separately).
-- Returns [] when no active workflow exists for the doctype.
-- ----------------------------------------------------------------------------
DEFINE FUNCTION OVERWRITE fn::workflow::get_transitions(
    $user:          string,
    $doctype:       string,
    $current_state: string
) {
    -- Find active workflow for this doctype
    LET $wf = SELECT name, workflow_state_field
              FROM tabWorkflow
              WHERE document_type = $doctype
                AND (is_active = true OR is_active = 1)
              LIMIT 1;

    IF array::len($wf) = 0 { RETURN []; };

    LET $wf_name = $wf[0].name;

    -- Return all transitions from the current state
    RETURN SELECT name, state, action, next_state, allowed, condition
           FROM tabWorkflowTransition
           WHERE parent      = $wf_name
             AND parenttype  = 'Workflow'
             AND state       = $current_state;
};

-- ----------------------------------------------------------------------------
-- fn::workflow::apply($user, $doctype, $doc_id, $action) → object
--
-- Applies a workflow action to a document atomically:
--   1. Finds the active workflow for $doctype
--   2. Reads the document's current workflow_state
--   3. Finds the matching transition (action + current_state)
--   4. Updates the workflow_state field (and docstatus when defined)
--   5. Returns the updated document
--
-- Throws on missing workflow, unknown action, or no valid transition.
-- ----------------------------------------------------------------------------
DEFINE FUNCTION OVERWRITE fn::workflow::apply(
    $user:    string,
    $doctype: string,
    $doc_id:  string,
    $action:  string
) {
    -- 1. Find active workflow
    LET $wf = SELECT name, workflow_state_field
              FROM tabWorkflow
              WHERE document_type = $doctype
                AND (is_active = true OR is_active = 1)
              LIMIT 1;

    IF array::len($wf) = 0 {
        THROW "No active workflow found for doctype '" + $doctype + "'";
    };

    LET $wf_name     = $wf[0].name;
    LET $state_field = $wf[0].workflow_state_field ?? 'workflow_state';

    -- 2. Read current state from the document
    LET $doc = SELECT * FROM type::table("tab" + $doctype)
               WHERE name = $doc_id
               LIMIT 1;

    IF array::len($doc) = 0 {
        THROW "Document '" + $doc_id + "' not found in " + $doctype;
    };

    LET $current_state = <string>($doc[0][$state_field] ?? '');

    -- 3. Find matching transition
    LET $transition = SELECT next_state
                     FROM tabWorkflowTransition
                     WHERE parent     = $wf_name
                       AND state      = $current_state
                       AND action     = $action
                     LIMIT 1;

    IF array::len($transition) = 0 {
        THROW "No transition found for action '" + $action
              + "' from state '" + $current_state + "'";
    };

    LET $next_state = $transition[0].next_state;

    -- 4. Look up doc_status for the next state (optional)
    LET $state_def = SELECT doc_status
                    FROM tabWorkflowDocumentState
                    WHERE parent = $wf_name
                      AND state  = $next_state
                    LIMIT 1;

    -- 5. Apply the state transition
    IF array::len($state_def) > 0 AND $state_def[0].doc_status != NONE {
        UPDATE type::table("tab" + $doctype)
            SET ($state_field) = $next_state,
                docstatus = <int>$state_def[0].doc_status
            WHERE name = $doc_id;
    } ELSE {
        UPDATE type::table("tab" + $doctype)
            SET ($state_field) = $next_state
            WHERE name = $doc_id;
    };

    -- 6. Return the updated document
    RETURN (SELECT * FROM type::table("tab" + $doctype)
            WHERE name = $doc_id LIMIT 1)[0];
};

-- Universal node: fn::on_submit::mark_submitted
--
-- Sets docstatus = 1 and status = "Submitted" on the document.
-- This is always the LAST node (ord=99) in every submittable doctype's
-- submit/on_submit stage, exactly mirroring fn::on_cancel::mark_cancelled
-- which sets docstatus = 2 and status = "Cancelled".
--
-- Because this runs inside fn::pipeline::run (a single SurrealDB statement),
-- the UPDATE here is part of the same implicit transaction as all preceding
-- GL/sub-document inserts.  If a THROW occurs anywhere in the pipeline call,
-- this UPDATE is rolled back together with every other mutation in that call.
DEFINE FUNCTION OVERWRITE fn::on_submit::mark_submitted(
    $doc_id: record,
    $config: object
) {
    UPDATE $doc_id SET
        docstatus = 1,
        status    = "Submitted";
    RETURN { ok: true };
};

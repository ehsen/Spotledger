-- Universal node: fn::on_cancel::mark_cancelled
--
-- Sets docstatus = 2 and status = "Cancelled" on the document.
-- This is always the last node in any cancel pipeline's on_cancel stage.
DEFINE FUNCTION OVERWRITE fn::on_cancel::mark_cancelled(
    $doc_id: record,
    $config: object
) {
    UPDATE $doc_id SET
        docstatus = 2,
        status    = "Cancelled";
    RETURN { ok: true };
};
